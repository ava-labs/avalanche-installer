
Avalanche installer

![Crates.io](https://img.shields.io/crates/v/avalanche-installer?logo=rust&style=for-the-badge)

https://crates.io/crates/avalanche-installer

Automates:

```bash
VERSION=1.9.7
rm -rf /tmp/avalanchego.tar.gz /tmp/avalanchego-v${VERSION}
curl -L ${DOWNLOAD_URL}/v${VERSION}/avalanchego-linux-amd64-v${VERSION}.tar.gz -o /tmp/avalanchego.tar.gz
tar xzvf /tmp/avalanchego.tar.gz -C /tmp
find /tmp/avalanchego-v${VERSION}
```
