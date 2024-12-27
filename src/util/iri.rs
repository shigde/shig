#[derive(Debug, Clone)]
pub struct IriSet {
    pub actor: String,
    pub following: String,
    pub followers: String,
    pub outbox: String,
    pub inbox: String,
    pub shared_inbox: String,
}

impl IriSet {
    pub fn new(actor: &str, domain: &str, tls: bool) -> Self {
        let actor = build_actor_iri(actor, domain, tls);
        let following = format!("{}/following", actor);
        let followers = format!("{}/followers", actor);
        let outbox = format!("{}/outbox", actor);
        let inbox = format!("{}/inbox", actor);
        let shared_inbox = build_shared_inbox_iri(domain, tls);
        Self { actor, following, followers, outbox, inbox, shared_inbox }
    }
}

pub fn build_actor_iri(actor: &str, domain: &str, tls: bool) -> String {
    let http = if tls { "https" } else { "http" };
    format!("{}://{}/federation/accounts/{}", http, domain, actor)
}

#[allow(dead_code)]
pub fn build_following_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/following", build_actor_iri(actor, domain, tls))
}

#[allow(dead_code)]
pub fn build_followers_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/followers", build_actor_iri(actor, domain, tls))
}

#[allow(dead_code)]
pub fn build_inbox_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/inbox", build_actor_iri(actor, domain, tls))
}

#[allow(dead_code)]
pub fn build_outbox_iri(actor: &str, domain: &str, tls: bool) -> String {
    format!("{}/outbox", build_actor_iri(actor, domain, tls))
}

pub fn build_shared_inbox_iri(domain: &str, tls: bool) -> String {
    let http = if tls { "https" } else { "http" };
    format!("{}://{}/federation/inbox", http, domain)
}
