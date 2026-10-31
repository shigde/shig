use super::*;

#[tokio::test]
async fn recording_does_not_report_success_without_an_archive_writer() {
    let service = MediaService::default();
    let id = uuid::Uuid::new_v4().to_string();
    assert!(service
        .start(&id)
        .await
        .unwrap_err()
        .to_string()
        .contains("not live"));
    service.live_started(&id).await.unwrap();
    assert!(service
        .start(&id)
        .await
        .unwrap_err()
        .to_string()
        .contains("not connected"));
    assert_eq!(service.stop(&id).await.unwrap().status, "empty");
    assert!(service.publish(&id, true).await.is_err());
    service.live_stopped(&id).await;
    assert!(service
        .start(&id)
        .await
        .unwrap_err()
        .to_string()
        .contains("not live"));
}

#[tokio::test]
async fn storage_selection_and_controls_do_not_create_files() {
    let root = std::env::temp_dir().join(format!("shig-media-test-{}", uuid::Uuid::new_v4()));
    let service = MediaService::with_htdocs(Config::default(), root.to_str().unwrap());
    assert_eq!(service.storage_directory(), root.join("recordings"));
    let id = uuid::Uuid::new_v4().to_string();
    service.status(&id).await.unwrap();
    service.stop(&id).await.unwrap();
    service.publish(&id, false).await.unwrap();
    assert!(!root.exists());
    let override_root = root.join("custom");
    let overridden = MediaService::new(Config {
        directory: Some(override_root.clone()),
    });
    assert_eq!(overridden.storage_directory(), override_root);
    assert!(!root.exists());
}
