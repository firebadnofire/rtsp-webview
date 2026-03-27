# RTSP Viewer (Rust + WebView) — RC1 Spec for Agentic Implementation (Updated)

## Key Change: Startup and Bulk Autoconfiguration

The application no longer ships with a predefined screen layout.

### Startup Behavior

- On first launch the application contains **no screens and no panels**.
- The UI presents an empty workspace with a prompt to either:
  - manually create cameras
  - run the **Bulk Autoconfiguration Tool**.

Screens are created automatically when cameras are generated.

---

## Bulk Autoconfiguration Tool

The bulk autoconfiguration tool allows rapid setup of large RTSP camera installations.

The tool works by accepting a **template RTSP URL** containing placeholder variables.

Example template:

rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum

Users may modify the template freely. The system only interprets placeholder variables and does not assume any fixed path structure.

This allows compatibility with many camera vendors whose RTSP path differs.

---

## Supported Placeholder Variables

The following variables are recognized in the template string:

$USERNAME
$PASSWORD
$IP
$PORT
$cameraNum
$subNum

Variables may appear anywhere in the URL.

Example valid custom template:

rtsp://$USERNAME:$PASSWORD@$IP:$PORT/live/channel$cameraNum/sub$subNum

---

## Autoconfiguration Inputs

The bulk configuration UI requests the following fields:

Username
Password
Camera IP
Port
Camera channel range
Subtype range

Channel range example:

1-16

Subtype range example:

0-1

The system iterates through these ranges to generate cameras.

---

## Generation Algorithm

For each camera channel value in the specified range:

For each subtype value in the subtype range:

1. Substitute template variables.
2. Generate a camera panel.
3. Assign the panel to the next available grid slot.

Panels are grouped into screens automatically.

Each screen contains **four panels (2×2 grid)**.

When four panels are filled, a new screen is created.

Example:

Channel range: 1-16
Subtype range: 0

Result:

16 cameras generated
4 screens created
4 panels per screen

---

## Post Generation Behavior

After generation:

- All cameras appear in the configuration.
- Screens are automatically created.
- Credentials are stored in the secret store.
- Panels are ready to start streaming.

Users may still edit individual panel settings afterward.

---

## Design Goals

The bulk autoconfiguration system must:

- support large installations quickly
- avoid vendor lock-in
- require minimal manual entry
- remain compatible with arbitrary RTSP path formats

The system must treat the template URL as opaque except for variable substitution.

---

## Example

Template:

rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor?channel=$cameraNum&subtype=$subNum

Inputs:

Username: admin
Password: password123
IP: 192.168.1.10
Port: 554
Channel range: 1-16
Subtype range: 0

Resulting generated URLs include:

rtsp://admin:password123@192.168.1.10:554/cam/realmonitor?channel=1&subtype=0
rtsp://admin:password123@192.168.1.10:554/cam/realmonitor?channel=2&subtype=0
...
rtsp://admin:password123@192.168.1.10:554/cam/realmonitor?channel=16&subtype=0

These cameras are distributed into screens automatically.


---

## URL Encoding Requirements

User-provided input values that are inserted into RTSP templates **must be URL-encoded before substitution**.

This prevents malformed URLs when credentials contain reserved characters.

Examples of characters that must be encoded include but are not limited to:

@ : / ? # [ ]

Common problematic cases:

Passwords containing `@`
Passwords containing `:`
Passwords containing `/`

Example:

Input:

Password: p@ss:word

Raw substitution would produce:

rtsp://admin:p@ss:word@192.168.1.10:554/...

This is invalid because `@` and `:` break URL parsing.

Correct behavior:

Encode credentials before insertion.

Encoded password:

p%40ss%3Aword

Final URL:

rtsp://admin:p%40ss%3Aword@192.168.1.10:554/...

Implementation requirement:

- Apply URL encoding to `$USERNAME` and `$PASSWORD` variables before template substitution.
- Do **not** encode the entire URL string.
- Only encode the user-provided values.

---

## Development Safety Requirement

A common failure mode during development is launching the Tauri window while the frontend has not loaded correctly, producing a **blank white window** when running `cargo run`.

RC1 must explicitly prevent this condition.

Requirements:

1. The application must verify that the frontend bundle is present before opening the window.

2. If the UI fails to load, the program must display a clear diagnostic error instead of showing a blank window.

3. During development builds (`cargo run`), the application should fail fast if:

- the frontend dev server is not reachable
- the UI bundle path is incorrect
- the UI build directory is empty

Recommended behavior:

If the frontend fails to load, the application must either:

- terminate with an explicit error message, or
- open a diagnostic window explaining the failure.

The system must **never silently open a blank UI window**.

This requirement exists because blank-window failures waste development time and create false debugging paths.

---

## Local Development RTSP Test Server

A local RTSP test server is available for development and testing purposes.

Address:

127.0.0.1

Example stream URL:

rtsp://test:testpw3%40000@127.0.0.1:5554/cam/realmonitor/cam/realmonitor?channel=11&subtype=0

Notes:

- The raw password contains the character `@` and must therefore be URL-encoded.
- Actual password: `testpw3@000`
- Encoded password used in URLs: `testpw3%40000`

Capabilities of the test server:

- Up to **16 channels** are available.
- Each channel supports **two subtypes**:
  - `subtype=0` → primary stream (higher resolution)
  - `subtype=1` → secondary stream (lower resolution)

Valid channel range:

1–16

Example generated URLs:

rtsp://test:testpw3%40000@127.0.0.1:5554/cam/realmonitor/cam/realmonitor?channel=1&subtype=0
rtsp://test:testpw3%40000@127.0.0.1:5554/cam/realmonitor/cam/realmonitor?channel=1&subtype=1
rtsp://test:testpw3%40000@127.0.0.1:5554/cam/realmonitor/cam/realmonitor?channel=2&subtype=0
...

Development requirement:

- The programmer may access this server at any time during development.
- It should be used as the **primary integration test source** for validating:

  - RTSP connectivity
  - authentication handling
  - URL encoding
  - multi-camera generation
  - reconnect logic
  - snapshot functionality

The bulk autoconfiguration feature should be able to generate a full configuration for this server using:

Template:

rtsp://$USERNAME:$PASSWORD@$IP:$PORT/cam/realmonitor/cam/realmonitor?channel=$cameraNum&subtype=$subNum

Inputs:

Username: test
Password: testpw3@000
IP: 127.0.0.1
Port: 5554
Channel range: 1-16
Subtype range: 0-1

Expected result:

32 generated panels (16 channels × 2 subtypes).

Panels must then be distributed automatically across screens (4 panels per screen).
