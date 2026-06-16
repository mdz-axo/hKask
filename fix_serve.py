#!/usr/bin/env python3
"""Fix serve.rs REQ tag."""

import re

filepath = "crates/hkask-cli/src/commands/serve.rs"
pattern = r"^pub async fn run_server\(port: u16, host: &str\) -> Result<\(\), Box<dyn std::error::Error>> \{"
req = "CLI-087"
pre = "port is a valid u16; host is a non-empty bind address string"
post = "starts the HTTP API server on the given host:port; returns Ok(()) on successful bind or Error on failure"

with open(filepath, "r") as f:
    content = f.read()

req_block = f"/// REQ: {req}\n/// pre:  {pre}\n/// post: {post}\n"

match = re.search(pattern, content, re.MULTILINE)
if not match:
    print(f"WARNING: Pattern not found")
    # Try a simpler pattern
    pattern2 = r"pub async fn run_server\(port: u16, host: &str\)"
    match = re.search(pattern2, content, re.MULTILINE)
    if match:
        print(f"Found with simpler pattern at position {match.start()}")
        start = match.start()
        new_content = content[:start] + req_block + content[start:]
        with open(filepath, "w") as f:
            f.write(new_content)
        print(f"OK: Added REQ {req}")
else:
    start = match.start()
    new_content = content[:start] + req_block + content[start:]
    with open(filepath, "w") as f:
        f.write(new_content)
    print(f"OK: Added REQ {req}")
