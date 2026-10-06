<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- This file is 100% AI-generated (Claude Code, Claude Opus 5.5). -->

# Local patches to ace-server

Vendored from [samp-reston/ace](https://github.com/samp-reston/ace) at
`0706bef625fbde1d8fadbd84ea21bb8ee650f7e4` (`crates/ace-server`) and used via
`[patch]` in the workspace `Cargo.toml`. Sibling crates (`ace-core`, `ace-uds`,
`ace-sim`, `ace-proto`) are taken from the same git revision.

## Configurable outbox (saves ~120 KiB RAM on the AZ3166)

Upstream keeps a 16 x 4 KiB outbox inside `UdsServer`, and `drain_outbox`
copies it into a caller buffer of the same size: two 64 KiB buffers.

- `UdsServer<H, S, const Q: usize = MAX_OUTBOX>`: the outbox capacity is a
  const generic. The default keeps upstream behaviour; the `SimNode` impl is
  generic over `Q`.
- `UdsServer::drain_outbox_with(|dst, frame| ...)`: drains without a second
  buffer.

`Cargo.toml` was rewritten from workspace inheritance to explicit git
dependencies. No other source changes.
