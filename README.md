# rustline-pubip

Your public/WAN IP in your tmux status line, as a
[rustline](https://github.com/stevenwcarter/rustline) WASM plugin.

Fetches https://api.ipify.org (configurable) through rustline's TTL-cached,
capability-gated HTTP host function: the network is hit at most once per
`refresh_secs`, a stale answer is served while the endpoint is unreachable,
and the plugin can only reach URLs you explicitly allow.

## Install

    rustline plugin install stevenwcarter/rustline-pubip --name pubip
    rustline plugin approve pubip        # grants: https://api.ipify.org + https://api.ipify.org/*

(`--name pubip` is only needed on rustline v0.1.0, whose default install
name came from the repo; newer builds derive it from the release asset.)

The manifest requests two patterns rather than one `https://api.ipify.org*`
glob: `*` crosses every character (including `.`), so a single host* glob
would also match an attacker-registrable host like
`https://api.ipify.org.evil.example`. Splitting into the bare host (the
default fetch URL) plus a `/*` path pattern keeps the grant scoped to the
real host exactly.

Place it:

    rustline widget enable pubip

## Configure

    [plugins.pubip.options]
    # url = "https://api.ipify.org"   # any endpoint returning a plain-text IP
    # refresh_secs = 900              # host-side cache TTL
    # format = "󰩟 {ip}"
    # alt_format = ""                 # non-empty makes the widget click-toggleable
    # down_format = ""                # shown when no usable answer exists

A body that doesn't look like an IP (a captive portal's HTML page, say) is
treated as a failure — the widget renders `down_format`, never garbage.

## Building from source

    cargo build --release --target wasm32-unknown-unknown
    cp target/wasm32-unknown-unknown/release/pubip.wasm ~/.local/share/rustline/plugins/

The capability manifest (`manifest.toml`) is embedded into the `.wasm` as the
`rustline-manifest` custom section at build time, so `plugin approve` works
on the distributed binary alone. This repo doubles as a worked example of a
standalone (out-of-tree) rustline plugin: scaffolded with `rustline plugin
new`, SDK via a git dependency pinned to a rustline release tag, CI + tag
releases publishing the installable `.wasm`.

## License

MIT
