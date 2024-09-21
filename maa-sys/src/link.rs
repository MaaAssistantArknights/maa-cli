#[cfg(feature = "runtime")]
macro_rules! link {
    (
        $(
            pub fn $name:ident($($pname:ident: $pty:ty), * $(,)?)$(-> $ret:ty)*;
        )+
    ) => (
        use libloading::{Library, Symbol};

        #[allow(non_snake_case)]
        struct SharedLibrary {
            _handle: Library,
            $(
                $name: extern "C" fn($($pname: $pty), *) $(-> $ret)*,
            )+
        }


        impl SharedLibrary {
            pub fn new(path: impl AsRef<std::ffi::OsStr>) -> Result<Self, libloading::Error> {
                let handle = unsafe { libloading::Library::new(path)? };
                let lib = Self {
                    $(
                        $name: unsafe {
                            let symbol: Symbol<extern "C" fn($($pname: $pty), *) $(-> $ret)*> = handle.get(stringify!($name).as_bytes())?;
                            *symbol
                        },
                    )+
                    // We need to keep the handle alive, even though we don't use it.
                    _handle: handle,
                };
                Ok(lib)
            }

            $(
                #[allow(non_snake_case)]
                pub fn $name(&self, $($pname: $pty), *) $(-> $ret)* {
                    (self.$name)($($pname), *)
                }
            )+
        }

        use std::sync::RwLock;
        static SHARED_LIBRARY: RwLock<Option<SharedLibrary>> = RwLock::new(None);

        /// Load the shared library of MaaCore from the given path in this thread.
        pub fn load(path: impl AsRef<std::ffi::OsStr>) -> Result<(), libloading::Error> {
                let lib = SharedLibrary::new(path)?;
                *SHARED_LIBRARY.write().expect("Failed to lock shared library") = Some(lib);

                Ok(())
        }

        /// Unload the shared library of MaaCore in this thread.
        pub fn unload() {
            SHARED_LIBRARY.write().expect("Failed to lock shared library").take();
        }

        /// Check if the shared library of MaaCore is loaded in this thread.
        pub fn loaded() -> bool {
            SHARED_LIBRARY.read().expect("Failed to lock shared library").is_some()
        }

        $(
            /// See the documentation of safe wrapper function for usage.
            ///
            /// # Safety
            ///
            /// This function is unsafe because it calls a function from a shared library.
            ///
            /// # Panics
            ///
            /// This function will panic if the shared library is not loaded in this thread.
            #[allow(non_snake_case)]
            pub unsafe fn $name($($pname: $pty), *) $(-> $ret)* {
                match SHARED_LIBRARY.read().expect("Failed to lock shared library").as_ref() {
                    Some(lib) => lib.$name($($pname), *),
                    None => panic!("MaaCore is not loaded"),
                }
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
