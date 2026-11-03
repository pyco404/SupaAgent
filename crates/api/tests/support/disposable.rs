//! Exact disposable targets only; never accept a general application URL.
use sqlx::postgres::PgConnectOptions;
use supa_persistence::Store;

pub fn permitted(options: &PgConnectOptions) -> bool {
    let legacy = matches!(options.get_host(), "localhost" | "127.0.0.1")
        && options.get_port() == 55680
        && options.get_database() == Some("supa_m10_test");
    let isolated = options.get_socket().is_some_and(|socket| {
        socket.file_name().is_some_and(|s| s == "socket")
            && socket.parent().is_some_and(|base| {
                base.parent() == Some(std::path::Path::new("/tmp"))
                    && base
                        .file_name()
                        .is_some_and(|n| n.to_string_lossy().starts_with("supa-pg18-validation-"))
            })
    }) && options.get_port() == 55448
        && options.get_database() == Some("supa_pg18_suite_test");
    legacy || isolated
}

pub async fn connect(options: &PgConnectOptions) -> Store {
    assert!(
        permitted(options),
        "dedicated_disposable_work_database_required"
    );
    let url = sqlx::ConnectOptions::to_url_lossy(options).to_string();
    let store = Store::connect(&url)
        .await
        .expect("disposable_connection_failed");
    if options.get_database() == Some("supa_pg18_suite_test") {
        let socket = options.get_socket().unwrap();
        let base = socket.parent().unwrap();
        assert_eq!(base.canonicalize().unwrap(), base);
        let row: (String, String) = sqlx::query_as(
            "SELECT current_setting('data_directory'),current_setting('listen_addresses')",
        )
        .fetch_one(store.pool())
        .await
        .unwrap();
        assert_eq!(row.0, base.join("data").to_str().unwrap());
        assert!(row.1.is_empty(), "socket_only_disposable_cluster_required");
    }
    store
}

#[test]
fn rejects_wrong_database_network_port_and_primary_paths() {
    for url in [
        "postgresql://localhost/production?port=55680",
        "postgresql://remote/supa_m10_test?port=55680",
        "postgresql://localhost/supa_pg18_suite_test?port=55448",
        "postgresql://localhost/supa_pg18_suite_test?host=/home/pikoo/postgres-data/socket&port=55448",
    ] {
        assert!(!permitted(&url.parse().unwrap()));
    }
    assert!(permitted(
        &"postgresql://localhost/supa_m10_test?port=55680"
            .parse()
            .unwrap()
    ));
    assert!(permitted(&"postgresql://localhost/supa_pg18_suite_test?host=/tmp/supa-pg18-validation-fixture/socket&port=55448".parse().unwrap()));
}
