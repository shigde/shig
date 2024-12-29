pub fn build_domain_name(name: &str, domain: &str) -> String {
    format!("{}@{}", name, domain)
}

pub fn build_default_channel_name(name: &str) -> String {
    format!("channel_{}", name)
}

#[allow(dead_code)]
pub fn split_domain_name(domain_name: &str) -> (String, String) {
    #[allow(unused_variables)]
    if let [name, domain, other @ ..] = &domain_name
        .split("@")
        .map(String::from)
        .collect::<Vec<String>>()[..]
    {
        return (name.clone(), domain.clone());
    } else {
        return (String::from(domain_name), String::from("unknown"));
    }
}
