pub fn build_actor_iri(actor: &str, domain: &str, tls: bool) -> String {
    let http = if tls { "https" } else { "http" };
    format!("{}://{}/federation/accounts/{}", http, domain, actor)
}

pub fn build_following_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/following", build_actor_iri(actor, domain, tls))
}

pub fn build_followers_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/followers", build_actor_iri(actor, domain, tls))
}

pub fn build_inbox_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/inbox", build_actor_iri(actor, domain, tls))
}

pub fn build_outbox_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/outbox", build_actor_iri(actor, domain, tls))
}

pub fn build_shared_inbox_iri(domain: &str, tls: bool) -> String {
    let http = if tls { "https" } else { "http" };
    format!("{}://{}/federation/inbox", http, domain)
}
