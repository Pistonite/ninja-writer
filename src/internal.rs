#[macro_export]
#[doc(hidden)]
#[cfg(feature = "os-string")]
macro_rules! MaybeOs {
    (str) => {
        std::ffi::OsStr
    };
    (String) => {
        std::ffi::OsString
    };
    ($value:expr) => {
        std::ffi::OsString::from(AsRef::<str>::as_ref($value))
    };
}

#[macro_export]
#[doc(hidden)]
#[cfg(not(feature = "os-string"))]
macro_rules! MaybeOs {
    (str) => {
        str
    };
    (String) => {
        String
    };
    ($value:expr) => {
        $value
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! MaybeOsDisplay {
    ($value:expr) => {{
        let maybe_os_display_internal = $value;

        #[cfg(feature = "os-string")]
        let maybe_os_display_internal = maybe_os_display_internal.to_string_lossy();

        maybe_os_display_internal
    }};
}
