use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        x11: {all(unix, feature="x11")},
        win32: {windows},
    }
}