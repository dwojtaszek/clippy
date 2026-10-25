//! Downloads the price table and the Droid models page over HTTP.

use std::time::Duration;

/// The two texts the price refresh needs, as downloaded.
#[derive(Debug)]
pub struct Downloads {
    /// The price table, as JSON.
    pub table: String,
    /// The Droid models page, as Markdown.
    pub page: String,
}

/// Fetches the price table at `table_url`, then the models page at `page_url`. Each request may take at most `timeout`.
///
/// # Errors
///
/// Returns an error when either request fails, the server answers with an error status, the connection drops, or a request takes longer than `timeout`.
pub fn fetch_both(
    table_url: &str,
    page_url: &str,
    timeout: Duration,
) -> Result<Downloads, ureq::Error> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let table = fetch(&agent, table_url)?;
    let page = fetch(&agent, page_url)?;
    Ok(Downloads { table, page })
}

fn fetch(agent: &ureq::Agent, url: &str) -> Result<String, ureq::Error> {
    agent.get(url).call()?.body_mut().read_to_string()
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    use super::*;

    const LONG_TIMEOUT: Duration = Duration::from_secs(10);

    /// A loopback server that answers each request by its path. Unknown paths get a 404. The server stops with the test process.
    fn serve(routes: Vec<(&'static str, String)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let _ = thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let mut buf = [0_u8; 4096];
                let read = stream.read(&mut buf).unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..read]);
                let path = request.split_whitespace().nth(1).unwrap_or("");
                let response = routes.iter().find(|(route, _)| *route == path).map_or_else(
                    || response("404 Not Found", "no such page"),
                    |(_, r)| r.clone(),
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        base
    }

    /// A complete HTTP/1.1 response that closes the connection after its body.
    fn response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn both_bodies_come_back_in_order() {
        let base = serve(vec![
            ("/table", response("200 OK", r#"{"gpt-x": {}}"#)),
            (
                "/page",
                response("200 OK", "| GPT-X | `gpt-x` | 1× | `none` |"),
            ),
        ]);
        let downloads = fetch_both(
            &format!("{base}/table"),
            &format!("{base}/page"),
            LONG_TIMEOUT,
        )
        .unwrap();
        assert_eq!(
            downloads.table, r#"{"gpt-x": {}}"#,
            "the table response body is returned unchanged as the table"
        );
        assert_eq!(
            downloads.page, "| GPT-X | `gpt-x` | 1× | `none` |",
            "the page response body is returned unchanged as the page"
        );
    }

    #[test]
    fn an_error_status_on_the_table_is_an_error_and_names_the_status() {
        let base = serve(vec![
            ("/table", response("500 Internal Server Error", "oops")),
            ("/page", response("200 OK", "page")),
        ]);
        let err = fetch_both(
            &format!("{base}/table"),
            &format!("{base}/page"),
            LONG_TIMEOUT,
        )
        .unwrap_err();
        assert!(err.to_string().contains("500"), "{err}");
    }

    #[test]
    fn a_missing_page_fails_even_when_the_table_arrived() {
        let base = serve(vec![("/table", response("200 OK", "{}"))]);
        let err = fetch_both(
            &format!("{base}/table"),
            &format!("{base}/page"),
            LONG_TIMEOUT,
        )
        .unwrap_err();
        assert!(err.to_string().contains("404"), "{err}");
    }

    #[test]
    fn a_body_cut_short_by_the_server_is_an_error() {
        // Promises 500 bytes, sends 5, then closes the connection.
        let short = "HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\nshort";
        let base = serve(vec![
            ("/table", short.to_owned()),
            ("/page", response("200 OK", "page")),
        ]);
        let result = fetch_both(
            &format!("{base}/table"),
            &format!("{base}/page"),
            LONG_TIMEOUT,
        );
        assert!(result.is_err(), "a short body must not pass as a table");
    }

    #[test]
    fn a_connection_refused_is_an_error() {
        // Bind and drop a listener to get a port that nothing is listening on.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let url = format!("http://127.0.0.1:{port}/table");
        assert!(
            fetch_both(&url, &url, LONG_TIMEOUT).is_err(),
            "a refused connection to the table url is returned as an error"
        );
    }

    #[test]
    fn a_silent_server_hits_the_timeout() {
        // Accepts one connection, reads the request, and never answers until the test is done with it.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let (done, until_done) = mpsc::channel::<()>();
        let silent = thread::spawn(move || {
            if let Some(Ok(mut stream)) = listener.incoming().next() {
                let mut buf = [0_u8; 4096];
                let _ = stream.read(&mut buf);
                let _ = until_done.recv();
            }
        });
        let started = Instant::now();
        let result = fetch_both(
            &format!("{base}/table"),
            &format!("{base}/page"),
            Duration::from_millis(300),
        );
        let elapsed = started.elapsed();
        // Assert before joining: if fetch never connected, the server thread waits in accept and join would hang.
        assert!(
            result.is_err(),
            "a server that never answers makes the fetch fail rather than hang"
        );
        assert!(
            elapsed < Duration::from_secs(10),
            "gave up after {elapsed:?}"
        );
        drop(done);
        silent.join().unwrap();
    }
}
