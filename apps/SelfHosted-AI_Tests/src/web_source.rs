use crate::*;
use std::net::IpAddr;
fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !v.is_private()
                && !v.is_loopback()
                && !v.is_link_local()
                && !v.is_unspecified()
                && !v.is_broadcast()
                && !v.is_multicast()
                && v.octets()[0] != 0
        }
        IpAddr::V6(v) => {
            if let Some(ip) = v.to_ipv4_mapped() {
                public(IpAddr::V4(ip))
            } else {
                !v.is_loopback()
                    && !v.is_unspecified()
                    && !v.is_multicast()
                    && (v.segments()[0] & 0xfe00) != 0xfc00
                    && (v.segments()[0] & 0xffc0) != 0xfe80
            }
        }
    }
}
pub fn readable(html: &str) -> String {
    let mut s = html.to_string();
    for tag in ["script", "style", "noscript", "svg"] {
        let re = regex::Regex::new(&format!(r"(?is)<{tag}\b[^>]*>.*?</{tag}\s*>")).unwrap();
        s = re.replace_all(&s, "").into_owned();
    }
    s = regex::Regex::new(r"(?s)<!--.*?-->")
        .unwrap()
        .replace_all(&s, "")
        .into_owned();
    s = regex::Regex::new(r"(?is)</?(?:p|div|h[1-6]|li|br|section|article|tr)\b[^>]*>")
        .unwrap()
        .replace_all(&s, "\n")
        .into_owned();
    s = regex::Regex::new(r"(?s)<[^>]+>")
        .unwrap()
        .replace_all(&s, " ")
        .into_owned();
    for (a, b) in [
        ("&nbsp;", " "),
        ("&quot;", "\""),
        ("&#39;", "'"),
        ("&apos;", "'"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&amp;", "&"),
    ] {
        s = s.replace(a, b);
    }
    s = regex::Regex::new(r"&#(x[0-9a-fA-F]+|[0-9]+);")
        .unwrap()
        .replace_all(&s, |c: &regex::Captures| {
            let n = c[1]
                .strip_prefix('x')
                .map(|s| u32::from_str_radix(s, 16).ok())
                .unwrap_or_else(|| c[1].parse().ok());
            n.and_then(char::from_u32)
                .map(|c| c.to_string())
                .unwrap_or_default()
        })
        .into_owned();
    s.lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}
pub async fn fetch(address: &str) -> AResult<String> {
    let url = reqwest::Url::parse(address)?;
    ensure!(
        ["http", "https"].contains(&url.scheme())
            && url.username().is_empty()
            && url.password().is_none(),
        "Use a public HTTP(S) webpage URL without credentials"
    );
    let host = url.host_str().ok_or_else(|| anyhow!("URL needs a host"))?;
    let port = url.port_or_known_default().unwrap_or(443);
    let addresses = tokio::net::lookup_host((host, port))
        .await?
        .collect::<Vec<_>>();
    ensure!(!addresses.is_empty()&&addresses.iter().all(|a|public(a.ip())),"Webpage import accepts public hosts. Import your private notes through the NoteSorter connection or paste the page text.");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(30))
        .resolve_to_addrs(host, &addresses)
        .build()?;
    let mut r = http
        .get(url.clone())
        .header("user-agent", "StudyHub source importer")
        .send()
        .await?;
    ensure!(
        !r.status().is_redirection(),
        "This URL redirects. Paste its final browser address."
    );
    ensure!(r.status().is_success(), "Page returned HTTP {}", r.status());
    ensure!(
        r.content_length().unwrap_or(0) <= 2_000_000,
        "Page is too large; paste the relevant section instead"
    );
    let mut bytes = vec![];
    while let Some(chunk) = r.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 2_000_000,
            "Page is too large; paste the relevant section instead"
        );
        bytes.extend_from_slice(&chunk);
    }
    let text = readable(&String::from_utf8_lossy(&bytes));
    ensure!(
        !text.trim().is_empty(),
        "No readable page content; paste the selected page text instead"
    );
    Ok(format!("Source URL: {url}\n\n{text}"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn page_import_excludes_script_and_style_and_decodes_entities() {
        let s=readable("<style>hidden</style><script>secret()</script><h1>Algebra &amp; Geometry</h1><p>x&#178; &lt; 4</p>");
        assert!(s.contains("Algebra & Geometry"));
        assert!(s.contains("x² < 4"));
        assert!(!s.contains("secret"));
        assert!(!s.contains("hidden"));
    }
    #[test]
    fn private_and_metadata_addresses_are_not_public() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "169.254.169.254",
            "192.168.1.2",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
        ] {
            assert!(!public(ip.parse().unwrap()))
        }
        assert!(public("8.8.8.8".parse().unwrap()));
    }
}
