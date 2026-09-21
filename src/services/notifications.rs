pub fn stage_finished(title: &str, body: &str) -> Result<(), String> {
    notify_rust::Notification::new()
        .summary(title)
        .body(body)
        .sound_name("message-new-instant")
        .show()
        .map(|_| ())
        .map_err(|error| error.to_string())
}
