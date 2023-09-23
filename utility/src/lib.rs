pub mod audio;
pub mod ring_buffer;

#[allow(unused)]
pub const fn kilobytes(x: usize) -> usize {
    x * 1024
}

#[allow(unused)]
pub const fn megabytes(x: usize) -> usize {
    kilobytes(x) * 1024
}

#[allow(unused)]
pub const fn gigabytes(x: usize) -> usize {
    megabytes(x) * 1024
}

#[allow(unused)]
pub const fn terrabytes(x: usize) -> usize {
    gigabytes(x) * 1024
}
