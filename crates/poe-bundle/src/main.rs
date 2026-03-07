use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;
use poe_bundle::reader::BundleReader;
use poe_bundle::reader::BundleReaderRead;


fn main() {
    let ts = Instant::now();

    //let reader = BundleReader::from_install(Path::new(r#"/home/nihil/Games/path-of-exile/drive_c/Program Files (x86)/Grinding Gear Games/Path of Exile"#));
    let reader = BundleReader::from_install(Path::new(r#"/home/nihil/.local/share/Steam/steamapps/common/Path of Exile"#));
    let _size = reader.size_of("Data/Mods.dat").unwrap();

    println!("Read index: {} ms", ts.elapsed().as_millis());


    let wanted_files = [
        "Data/GemTags.dat",
        "Data/Tags.dat"
    ];

    let files: HashMap<_, _> = reader.index.paths.iter()
        .filter(|path| wanted_files.contains(&path.as_str()))
        .map(|path|
            //let output_target = format!("dat_dump/{path}");
            //let target_path = Path::new(&output_target).parent().unwrap();
            //print!("{:?}", output_target);
            //fs::create_dir_all(target_path).unwrap();
            //let mut f = fs::File::create(output_target).unwrap();
            //reader.write_into(path, &mut f);

            match reader.bytes(path) {
                Ok(bytes) => (path, bytes),
                Err(_) => panic!()
            }
        )
        .collect();

    for (file, bytes) in files {
        println!("Got {} bytes for {}", bytes.len(), file)
    }

    println!("Done: {} ms", ts.elapsed().as_millis());
    //let mut dst = Vec::with_capacity(size);
    //reader.write_into("Data/Mods.dat", &mut dst).unwrap();
    //println!("got mods data {}", dst.len())

    //let mut f = File::create("Mods.dat").unwrap();

    //reader.write_into("Data/Mods.dat", &mut f);


}