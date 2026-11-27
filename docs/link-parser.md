# 🔗 Link parser

[← All documentation](README.md)

`POST /api/filaments/parse-url` takes a product link and returns a draft of the filament's details. It never fails just because data is missing. It returns what it found, the fields it did not find, where each value came from, and warnings in plain language. The **+ Add spool** form uses it.

## 🪜 The steps

The parser runs these steps from the most to the least trustworthy. Each field gets its value from the first step that finds one.

1. **Safety check.** Only `http` and `https` links. A host that resolves to a private, loopback, or link-local address is refused, and so is a redirect to one. If the host can't be resolved, the parser skips the fetch and uses only the link itself.
2. **Fetch.** A browser-like user agent, a 12 second timeout, a 4 MB limit, and up to 5 redirects. Captcha and "access denied" pages (the Amazon robot check, a Cloudflare challenge, HTTP 403, 429, or 503) are marked as blocked.
3. **Shopify product data.** Many filament stores run on Shopify (Polymaker, eSUN, HATCHBOX, and more). Their `/products/<handle>.js` gives the vendor, the title, the selected color variant, the price, and the description.
4. **schema.org data** (`Product` in JSON-LD).
5. **Amazon** pages: the title, the brand line, the overview and technical tables, the bullet points, the image, and the price.
6. **Spec tables** on any page: `<table>` rows, `<dl>` lists, and "Key: Value" bullets.
7. **OpenGraph and meta tags.**
8. **The title**: brand, material, color name, weight, and diameter.
9. **The page text**: temperature ranges, speeds, and drying conditions.
10. **The optional LLM step**, only when `ANTHROPIC_API_KEY` is set and important fields are still empty. It only fills gaps.
11. **The link itself**: the brand from the store's domain, and the material and color from the path.
12. **Derived values**: a swatch color from the color name, the usual spelling of the brand, and a short product name.

## 🧱 When a store blocks the page

Amazon and some other stores often answer automated requests with a captcha. Then the parser still returns what it can get from the link (usually the brand, and sometimes the material and color) and a warning. Fill in the rest of the form by hand, or paste the manufacturer's own product page instead.

## 🧪 Trying it on a saved page

To see what the parser finds in a page that you saved from your browser:

```bash
cd backend
cargo run --example parse_file -- https://store.example/products/some-filament path/to/page.html
```
