use std::path::PathBuf;

pub fn exec(args: crate::args::ImportArgs) {
    if args.force {
        std::fs::remove_dir_all(&args.out_path).expect("couldn't remove");
        std::fs::create_dir(&args.out_path).expect("couldn't recreate")
    } else {
        crate::check_dir(&PathBuf::from(&args.out_path), false /* TODO */);
    }
}
