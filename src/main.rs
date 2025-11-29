#[cfg(windows)]
compile_error!(
    "Windows is not supported. Please use something that supports mmap(), i.e. Linux/macOS."
);

#[cfg(all(feature = "lexical-parse-float", feature = "fast-float"))]
compile_error!("Please enable only one of the features 'lexical-parse-float' and 'fast-float'.");

#[cfg(not(feature = "ahash"))]
type Map<K, V> = std::collections::HashMap<K, V>;

#[cfg(feature = "ahash")]
type Map<K, V> = ahash::AHashMap<K, V>;

mod mapped_file;
use mapped_file::MemoryMappedFile;

mod values;
use values::Values;

const FILENAME: &str = "measurements.txt";
const HASHMAP_CAPACITY: usize = 10_000;

fn merge_maps<'k>(merge_map: &mut Map<&'k str, Values>, mut map2: Map<&'k str, Values>) {
    for (key, value) in map2.drain() {
        merge_map
            .entry(key)
            .and_modify(|v| v.merge(&value))
            .or_insert_with(|| value);
    }
}

#[inline(always)]
fn parse_f32(s: &str) -> Option<f32> {
    #[cfg(feature = "lexical-parse-float")]
    {
        use lexical_parse_float::{format::STANDARD, parse::ParseFloat, Options};
        return f32::fast_path_complete::<{ STANDARD }>(s.as_bytes(), &Options::new()).ok();
    }

    #[cfg(feature = "fast-float")]
    return fast_float::parse(s).ok();

    #[cfg(not(any(feature = "lexical-parse-float", feature = "fast-float")))]
    return s.parse().ok();
}

fn main() {
    // Open file
    let file = MemoryMappedFile::new(std::path::Path::new(FILENAME)).expect("Unable to open file");

    // Split the file into chunks
    let n_cores = std::thread::available_parallelism().unwrap().get();
    let bytes_per_core = file.len() / n_cores;

    let mut start = 0usize;
    let mut end = bytes_per_core;

    std::thread::scope(|scope| {
        // Thread handles, we need to keep them to retrieve the results
        let mut handles = Vec::with_capacity(n_cores);

        // Find line boundaries
        for _core_id in 0..n_cores {
            while file.get(end) != Some(&b'\n') {
                end += 1;
                if end >= file.len() {
                    end = file.len();
                    break;
                }
            }

            // eprintln!(
            //     "Core {_core_id}: Start: {start}, End: {end}, Size: {}",
            //     end - start
            // );

            // Spawn thread
            handles.push(scope.spawn(move || {
                // Local map for this thread
                let mut map = Map::<&'static str, Values>::with_capacity(HASHMAP_CAPACITY);

                // Open a local view of the file for this thread, seems to be faster than if all threads access the same memory mapped file.
                let local_file = MemoryMappedFile::new(std::path::Path::new(FILENAME))
                    .expect("Unable to open file");
                let local_data_str =
                    unsafe { std::str::from_utf8_unchecked(&local_file[start..end]) };

                // Process chunk, line by line
                for line in local_data_str.lines() {
                    unsafe {
                        if let Some((city, Some(temp))) = line
                            // We know that the temperature is always at least 3 bytes, we should move back from the end by a constant amount before seeking the semicolon.
                            .get_unchecked(..(line.len() - 3))
                            .rfind(';')
                            // SAFETY: We know that 'mid' is a valid index because we just found it by searching for ';'.
                            .map(|mid| (line.get_unchecked(..mid), line.get_unchecked(mid..)))
                            .map(|(city, temp_str)| (city, parse_f32(temp_str.get_unchecked(1..))))
                        {
                            // Aaah, yes. Promote the lifetime of the city to 'static. This is **fine** as long as local_file is not dropped.
                            map.entry(core::mem::transmute::<&str, &'static str>(city))
                                .and_modify(|values| values.add(temp))
                                .or_insert_with(|| Values::new(temp));
                        } else {
                            eprintln!("Invalid line: {}", line);
                        }
                    }
                }
                (map, local_file)
            }));

            // Move to next chunk
            start = end + 1;
            end = start + bytes_per_core;
        }

        // Merge results from threads
        let mut map = Map::<&str, Values>::with_capacity(HASHMAP_CAPACITY);
        // Make sure that we keep the mmapped files alive until we're done with the results
        let mut mmapped_files = Vec::with_capacity(n_cores);
        for handle in handles {
            let (handle_map, mmapped_file) = handle.join().unwrap();
            merge_maps(&mut map, handle_map);
            mmapped_files.push(mmapped_file);
        }

        // It's faster to use a HashMap and sort the result than to use a BTreeMap.
        let mut key_values_pairs = map.drain().collect::<Vec<(_, _)>>();
        key_values_pairs.sort_unstable_by(|(a, _), (b, _)| (*a).partial_cmp(b).unwrap());

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
