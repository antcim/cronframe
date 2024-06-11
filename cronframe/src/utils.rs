use rand::distributions::DistString;

pub fn generate_id(len: usize) -> String {
    rand::distributions::Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}
