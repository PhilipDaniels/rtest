pub mod configuration;
pub mod engine;
#[path = "jobs/jobs.rs"]
pub mod jobs;
pub mod shadow_copy_destination;
pub mod source_directory_watcher;
pub mod state;
mod thread_clutch;
mod utils;


// TODO: Why does adding this cause an error in mismatch count?

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }
