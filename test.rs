use std::fs;

fn main() {
	dbg!(fs::read_dir("src").unwrap().next().unwrap().unwrap().file_type().unwrap());
	
}
