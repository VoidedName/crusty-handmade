#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub fn platform_main() {
    use self::windows::win32main::win32main;

    win32main()
}
