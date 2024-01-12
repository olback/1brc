#[cfg(windows)]
compile_error!("Windows is not supported. Please use something that supports mmap().");

use std::collections::HashMap;

mod mapped_file;
use mapped_file::MemoryMappedFile;

mod values;
use values::Values;

const FILENAME: &str = "measurements.txt";
const HASHMAP_CAPACITY: usize = 10_000;

fn merge_maps<'k>(merge_map: &mut HashMap<&'k str, Values>, map2: &HashMap<&'k str, Values>) {
    for (key, value) in map2.iter() {
        merge_map
            .entry(*key)
            .and_modify(|v| v.merge(value))
            .or_insert_with(|| value.clone());
    }
}

fn main() {
    // Open file
    let file = MemoryMappedFile::new(std::path::Path::new(FILENAME)).expect("Unable to open file");

    // Assume that the file is UTF-8
    let data_str = unsafe { std::str::from_utf8_unchecked(&file) };

    // Split the file into chunks
    let n_cores = std::thread::available_parallelism().unwrap().get();
    let bytes_per_core = file.len() / n_cores;

    let mut start = 0usize;
    let mut end = bytes_per_core;

    std::thread::scope(|scope| {
        // Thread handles, we need to keep them to retrieve the results
        let mut handles = Vec::with_capacity(n_cores);

        // Find line boundaries
        for core_id in 0..n_cores {
            while file.get(end) != Some(&b'\n') {
                end += 1;
                if end >= file.len() {
                    end = file.len();
                    break;
                }
            }

            eprintln!(
                "Core {core_id}: Start: {start}, End: {end}, Total: {}",
                end - start
            );

            // Spawn thread
            handles.push(scope.spawn(move || {
                // Local map for this thread
                let mut map = HashMap::<&str, Values>::with_capacity(HASHMAP_CAPACITY);

                // Process chunk, line by line
                let data = &data_str[start..end];
                for line in data.lines() {
                    unsafe {
                        if let Some((city, Some(temp))) = line
                            // We know that the temperature is always at least 3 bytes, we should move back from the end by a constant amount before seeking the semicolon.
                            .get_unchecked(..(line.len() - 3))
                            .rfind(';')
                            // SAFETY: We know that 'mid' is a valid index because we just found it by searching for ';'.
                            .map(|mid| (line.get_unchecked(..mid), line.get_unchecked(mid..)))
                            .map(|(city, temp_str)| {
                                (city, temp_str.get_unchecked(1..).parse().ok())
                            })
                        {
                            map.entry(city)
                                .and_modify(|values| values.add(temp))
                                .or_insert_with(|| Values::new(temp));
                        } else {
                            eprintln!("Invalid line: {}", line);
                        }
                    }
                }
                map
            }));

            // Move to next chunk
            start = end + 1;
            end = start + bytes_per_core;
        }

        // Merge results from threads
        let mut map = HashMap::<&str, Values>::with_capacity(HASHMAP_CAPACITY);
        for handle in handles {
            let handle_map = handle.join().unwrap();
            merge_maps(&mut map, &handle_map);
        }

        // It's faster to use a HashMap and sort the result than to use a BTreeMap.
        let mut key_values_pairs = map.drain().collect::<Vec<(_, _)>>();
        key_values_pairs.sort_unstable_by(|(a, _), (b, _)| (*a).partial_cmp(*b).unwrap());

        for (city, values) in key_values_pairs.iter() {
            println!(
                "{:20} min: {:5.1}°C, max: {:5.1}°C, mean: {:5.1}°C",
                city,
                values.min,
                values.max,
                values.mean()
            );
        }
    });
}
