use std::io;
use std::fs;
use std::clone;

pub enum Input {
    File(fs::File),
    Cursor(io::Cursor<Box<[u8]>>),
}

pub enum Output {
    File(fs::File),
    Cursor(io::Cursor<Vec<u8>>),
    Standard(io::Stdout),
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            Input::File(ref mut f)   => f.read(buf),
            Input::Cursor(ref mut c) => c.read(buf),
        }
    }
}

impl io::Seek for Input {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match *self {
            Input::File(ref mut f)   => f.seek(pos),
            Input::Cursor(ref mut c) => c.seek(pos),
        }
    }
}

impl io::Write for Output {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            Output::File(ref mut f)     => f.write(buf),
            Output::Cursor(ref mut c)   => c.write(buf),
            Output::Standard(ref mut s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            Output::File(ref mut f)     => f.flush(),
            Output::Cursor(ref mut _c)  => Ok(()),
            Output::Standard(ref mut s) => s.flush(),
        }
    }
}

impl clone::Clone for Input {
    fn clone(&self) -> Input {
        match *self {
            Input::File(ref f)   => Input::File(f.try_clone().unwrap()),
            Input::Cursor(ref c) => Input::Cursor(c.clone()),
        }
    }
}

impl clone::Clone for Output {
    fn clone(&self) -> Output {
        match *self {
            Output::File(ref f)      => Output::File(f.try_clone().unwrap()),
            Output::Cursor(ref c)    => Output::Cursor(c.clone()),
            Output::Standard(ref _s) => Output::Standard(io::stdout()),
        }
    }
}
