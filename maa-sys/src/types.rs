pub enum StaticOptionKey {
    Invalid = 0,
}

pub enum InstanceOptionKey {
    Invalid = 0,
    // MinitouchEnabled = 1, // Deprecated
    TouchMode = 2,
    DeploymentWithPause = 3,
    AdbLiteEnabled = 4,
    KillAdbOnExit = 5,
}

pub enum TouchMode {
    Abd = 0,
    Minitouch = 1,
    Maatouch = 2,
    MacPlayTools = 3,
}
