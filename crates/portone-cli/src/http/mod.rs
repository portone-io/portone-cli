pub fn build_agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(None)
        .redirect_auth_headers(ureq::config::RedirectAuthHeaders::SameHost)
        .tls_config(tls)
        .build();
    ureq::Agent::new_with_config(config)
}
