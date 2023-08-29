#[cfg(feature = "runtime")]
#[macro_export]
macro_rules! link {
    (
        $(
            pub fn $name:ident($($pname:ident: $pty:ty), * $(,)?)$(-> $ret:ty)*;
        )+
    ) => (
        use libloading::{Library, Symbol};

        #[allow(non_snake_case)]
        struct SharedLibrary {
            handle: Library,
            $(
                $name: Option<extern "C" fn($($pname: $pty), *) $(-> $ret)*>,
            )+
        }


        impl SharedLibrary {
            pub fn new(path: impl AsRef<std::ffi::OsStr>) -> Result<Self, libloading::Error> {
                let handle = unsafe { libloading::Library::new(path)? };
                let mut lib = Self {
                    handle,
                    $(
                        $name: None,
                    )+
                };
                unsafe {
                    $(
                        let symbol: Symbol<extern "C" fn($($pname: $pty), *) $(-> $ret)*> = lib.handle.get(stringify!($name).as_bytes())?;
                        lib.$name = Some(*symbol);
                    )+
                }
                Ok(lib)
            }

            $(
                #[allow(non_snake_case)]
                pub fn $name(&self, $($pname: $pty), *) $(-> $ret)* {
                    match self.$name {
                        Some(f) => f($($pname), *),
                        None => panic!("Function {} is not loaded.", stringify!($name)),
                    }
                }
            )+
        }

        use std::cell::RefCell;
        use std::sync::Arc;

        thread_local! {
            static SHARED_LIBRARY: RefCell<Option<Arc<SharedLibrary>>> = RefCell::new(None);
        }

        pub fn load(path: impl AsRef<std::ffi::OsStr>) {
                SHARED_LIBRARY.with(|lib| {
                    *lib.borrow_mut() = Some(Arc::new(SharedLibrary::new(path).unwrap()));
                });
        }

        pub fn unload() {
            SHARED_LIBRARY.with(|lib| {
                *lib.borrow_mut() = None;
            });
        }

        $(
            /// # Safety
            /// This function is unsafe because it calls a function from a shared library.
            #[allow(non_snake_case)]
            pub unsafe fn $name($($pname: $pty), *) $(-> $ret)* {
                SHARED_LIBRARY.with(|lib| match lib.borrow().as_ref() {
                    Some(lib) => lib.$name($($pname), *),
                    None => panic!("MaaCore is not loaded in this thread."),
                })
            }
        )+
    )
}

#[cfg(not(feature = "runtime"))]
#[macro_export]
macro_rules! link {
    (
        $(
            pub fn $name:ident($($pname:ident: $pty:ty), * $(,)?)$(-> $ret:ty)*;
        )+
    ) => (
        #[link(name = "MaaCore")]
        extern "C" {
            $(
                pub fn $name($($pname: $pty), *) $(-> $ret)*;
            )+
        }
    )
}
