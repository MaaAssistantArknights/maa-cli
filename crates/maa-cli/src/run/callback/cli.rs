use ratatui::{
    layout::{Constraint, Layout},
    prelude::CrosstermBackend,
    widgets::{Paragraph, Widget},
    TerminalOptions,
};

use super::summary::{SummarySubscriber, LINE_SEP};
use crate::run::callback::summary::TaskSummaryState;

#[tokio::main]
pub async fn entry(asst: &maa_sys::Assistant, rx: &mut SummarySubscriber) -> anyhow::Result<()> {
    let mut terminal = ratatui::Terminal::with_options(
        CrosstermBackend::new(std::io::stdout()),
        TerminalOptions {
            viewport: ratatui::Viewport::Inline(16),
        },
    )?;
    terminal.insert_before(1, |buf| Paragraph::new(LINE_SEP).render(buf.area, buf))?;
    let mut content = vec![];
    let mut roller = Roller::default();
    while asst.running() {
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => (),
            _ = wait_for_signal() => {
                let undones = rx.get_todo_tasks();
                terminal.insert_before(undones.lines().count() as u16, |buf| {
                    Paragraph::new(undones.as_str()).render(buf.area, buf)
                })?;
                terminal.clear()?;
                anyhow::bail!("Interrupted by user!")
            },
        };

        for state in rx.try_update().unwrap_or_default() {
            if let TaskSummaryState::End(task) = &state {
                terminal.insert_before(task.lines().count() as u16, |buf| {
                    Paragraph::new(task.as_str()).render(buf.area, buf)
                })?;
                terminal.insert_before(1, |buf| Paragraph::new(LINE_SEP).render(buf.area, buf))?;
            }
            content.extend(state.to_string().lines().map(|s| s.to_owned()));
        }

        let header = "Log:";
        let footer = rx.get_todo_task_names();
        terminal.autoresize()?;
        terminal.try_draw(|f| {
            let area = f.area();
            let split = Layout::vertical([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .split(area);

            let header_area = split[0];
            let header = Paragraph::new(header);
            f.render_widget(header, header_area);

            let para_area = split[1];
            let para = Paragraph::new(content.join("\n"))
                .scroll(((content.len() as u16).saturating_sub(para_area.height), 0));
            f.render_widget(para, para_area);

            let footer_area = split[2];
            let footer = Paragraph::new(format!(
                " {} Todo Tasks: {}",
                roller.next(),
                footer.join(", ")
            ));
            f.render_widget(footer, footer_area);

            Ok::<(), std::io::Error>(())
        })?;
    }
    terminal.clear()?;
    Ok(())
}

#[derive(Default)]
struct Roller {
    idx: u8,
}
impl Roller {
    pub fn next(&mut self) -> char {
        self.idx += 1;
        if self.idx == 4 {
            self.idx = 0
        }
        match self.idx {
            0 => '-',
            1 => '\\',
            2 => '|',
            3 => '/',
            _ => unreachable!(),
        }
    }
}

/// Waits for a signal that requests a graceful shutdown, like SIGTERM or SIGINT.
#[cfg(unix)]
async fn wait_for_signal_impl() {
    use tokio::signal::unix::{signal, SignalKind};

    // Infos here:
    // https://www.gnu.org/software/libc/manual/html_node/Termination-Signals.html
    let mut signal_terminate = signal(SignalKind::terminate()).unwrap();
    let mut signal_interrupt = signal(SignalKind::interrupt()).unwrap();
    let mut signal_quit = signal(SignalKind::quit()).unwrap();

    tokio::select! {
        _ = signal_terminate.recv() => log::debug!("Received SIGTERM."),
        _ = signal_interrupt.recv() => log::debug!("Received SIGINT."),
        _ = signal_quit.recv() => log::debug!("Received SIGQUIT."),
    };
}

/// Waits for a signal that requests a graceful shutdown, Ctrl-C (SIGINT).
#[cfg(windows)]
async fn wait_for_signal_impl() {
    use tokio::signal::windows;

    // Infos here:
    // https://learn.microsoft.com/en-us/windows/console/handlerroutine
    let mut signal_c = windows::ctrl_c().unwrap();
    let mut signal_break = windows::ctrl_break().unwrap();
    let mut signal_close = windows::ctrl_close().unwrap();
    let mut signal_shutdown = windows::ctrl_shutdown().unwrap();

    tokio::select! {
        _ = signal_c.recv() => log::debug!("Received CTRL_C."),
        _ = signal_break.recv() => log::debug!("Received CTRL_BREAK."),
        _ = signal_close.recv() => log::debug!("Received CTRL_CLOSE."),
        _ = signal_shutdown.recv() => log::debug!("Received CTRL_SHUTDOWN."),
    };
}

/// Registers signal handlers and waits for a signal that
/// indicates a shutdown request.
pub(crate) async fn wait_for_signal() {
    wait_for_signal_impl().await
}
