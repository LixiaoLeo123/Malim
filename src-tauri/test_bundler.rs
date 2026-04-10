use std::fs;
fn main() {
    let dict_path = "/home/leo/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rsmorphy-dict-ru-0.1.0/data";
    println!("ls: {:?}", fs::read_dir(dict_path).unwrap().count());
}
