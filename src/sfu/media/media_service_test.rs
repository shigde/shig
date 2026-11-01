use super::*;

#[tokio::test]
async fn legacy_recording_start_requires_the_recorded_video_endpoint() {
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
        .contains("Recorded Video endpoint"));
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

#[tokio::test]
async fn permanent_deletion_removes_only_the_selected_replay_and_can_be_retried() {
    let root = std::env::temp_dir().join(format!("shig-delete-test-{}", uuid::Uuid::new_v4()));
    let service = MediaService::new(Config {
        directory: Some(root.clone()),
    });
    let first = uuid::Uuid::new_v4().to_string();
    let second = uuid::Uuid::new_v4().to_string();
    for id in [&first, &second] {
        tokio::fs::create_dir_all(root.join(id).join("track-0/seg"))
            .await
            .unwrap();
        tokio::fs::write(root.join(id).join("track-0/seg/1.m4s"), b"segment")
            .await
            .unwrap();
        tokio::fs::write(root.join(id).join("master.m3u8"), b"playlist")
            .await
            .unwrap();
    }
    service.delete_recording_files(&first).await.unwrap();
    assert!(!root.join(&first).exists());
    assert!(root.join(&second).join("track-0/seg/1.m4s").exists());
    service.delete_recording_files(&first).await.unwrap();
    assert!(service.delete_recording_files("../other").await.is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join(&second), root.join(&first)).unwrap();
        service.delete_recording_files(&first).await.unwrap();
        assert!(root.join(&second).join("master.m3u8").exists());
    }
    tokio::fs::remove_dir_all(root).await.unwrap();
}
