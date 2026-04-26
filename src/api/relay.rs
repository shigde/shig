pub mod announcement;
pub mod tls;
pub mod media;
pub mod auth;

// List of available Streams / Broadcasts
// GET /announced (+ prefix)
// GET /announced/room/123

// [
// "room/123/video",
// "room/123/audio"
// ]

// HTTP-Fallback for Stream-Daten
// GET /fetch/{*path}
// GET /fetch/room/123/video


// HTTP (Control Plane)
// │
// ├── /certificate.sha256   → Trust
// ├── /announced            → Discovery
// ├── /fetch/...            → Fallback Data
// │
// ▼
// MoQ / QUIC (Data Plane)
// │
// └── echte Streams (UDP)