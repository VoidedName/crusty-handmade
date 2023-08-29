use std::path::Path;

#[cfg(windows)]
mod windows;

#[allow(unused)]
pub fn debug_platform_read_entire_file<C>(filename: C) -> std::io::Result<Vec<u8>>
where
    C: AsRef<Path>,
{
    std::fs::read(filename)
}

#[allow(unused)]
pub fn debug_platform_write_entire_file<C, D>(filename: C, data: D) -> std::io::Result<()>
where
    C: AsRef<Path>,
    D: AsRef<[u8]>,
{
    std::fs::write(filename, data)
}

#[cfg(windows)]
pub fn platform_main() {
    use self::windows::win32main::win32main;

    win32main()
}
