# ChefBar remote auth (private `*.online`)

ChefBar praat met private ChefGroep surfaces over **HTTPS** achter Cloudflare Access. Tailscale is optioneel (dev/bypass), nooit verplicht voor product-remote.

## Doel-architectuur

- **Edge:** Cloudflare Access op private `*.chefgroep.online` origins
- **Identity:** Authentik OIDC (doel) via dezelfde Access plane
- **Machine clients:** Cloudflare Access service tokens (`CF-Access-Client-Id` / `CF-Access-Client-Secret`) plus vault-api bearer waar nodig

## Interim (nu)

ChefBar gebruikt één seam: `chefbar.auth.get_headers()`.

| Mechanisme | Env vars |
| --- | --- |
| Vault-api bearer | `CHEF_VAULT_API_TOKEN` of `CHEFBAR_VAULT_TOKEN` (+ optioneel `CHEFBAR_VAULT_TOKEN_FILE`) |
| CF Access service token | `CF_ACCESS_CLIENT_ID` + `CF_ACCESS_CLIENT_SECRET` (of `CHEFBAR_CF_*`) |

Later: OIDC access tokens via dezelfde helper — clients hoeven niet opnieuw ontworpen te worden.

## Endpoint profile

Stel `~/.config/chefbar/endpoints.json` in (zie `config/endpoints.example.json`). Lokale dev blijft loopback; remote gebruikt `https://…chefgroep.online` hostnames — geen “poort 8321” in productcopy.

## Diagnose

```bash
chefbar --doctor
```

Per PROFILE-target: DNS, TLS, allowlist, latency, auth (401 vs 200). Fail-closed — geen “waarschijnlijk Tailscale”.

## Niet doen

- SSH-tunnels of Tailscale als verplichting documenteren
- Bearer tokens in git committen
- Plain HTTP naar publiek internet (policy blokkeert dit)
