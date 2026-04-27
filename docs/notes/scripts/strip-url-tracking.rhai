let text = clip["text"];
if text.starts_with("http") {
    clip["text"] = text.replace_all_regex("[?&](utm_[^=]+|fbclid)=[^&]*", "");
}
