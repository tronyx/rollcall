# 🔒 Security policy

## ✅ Supported versions

The project doesn't use version numbers. Only the latest release is supported: the `main` branch, and the `latest` Docker image on Docker Hub (`tronyx/rollcall`) and GitHub (`ghcr.io/tronyx/rollcall`). Fixes aren't made to older copies, so update to the latest before reporting (`git pull`, or `docker compose pull`).

The `develop` branch and image are the next release in progress. A vulnerability there is welcome too, since fixing it early keeps it out of the next release.

## 📮 Reporting a vulnerability

**Please don't open a public issue.** Report it privately instead, through GitHub: on the [Security tab](https://github.com/tronyx/rollcall/security), choose **Report a vulnerability**, or go straight to the [form](https://github.com/tronyx/rollcall/security/advisories/new). Only you and the maintainer can see the report.

It helps to include:

- what an attacker could do, and what they'd need first (for example, access to the web interface on your network);
- how to reproduce it: the request, the product link, or the steps in the web interface;
- how you run Rollcall (Docker image and its digest, `docker image inspect --format '{{.Id}}' tronyx/rollcall`, or the commit you built), and your PostgreSQL version.

If you can't use the form, [open an issue](https://github.com/tronyx/rollcall/issues) asking for a private way to get in touch, with no details of the problem.

## ⏱️ What to expect

This project is maintained by one volunteer in their spare time, so please allow for that:

- You'll get a reply within **7 days** saying whether the report is accepted.
- An accepted report is fixed privately, then released, with an advisory on the [Security tab](https://github.com/tronyx/rollcall/security) that credits you, unless you'd rather not be named. The aim is a release within **90 days** of the report, sooner for something serious, and you'll be kept up to date along the way.
- Please keep it private until the fix is released and the advisory published.

## 🎯 Scope

Rollcall is meant to run on a home network, without user accounts. Anyone who can reach the web interface can change the inventory, so don't expose it to the internet without a reverse proxy that adds authentication. These are in scope:

- **The link parser:** a link or a product page that makes the server fetch an address on your own network (the parser refuses private, loopback and link-local addresses unless `SCRAPER_ALLOW_PRIVATE_HOSTS` is on), read a local file, use too much memory or time, or run code. A page that returns wrong or missing data is a bug, not a vulnerability.
- **The API and the web interface:** SQL injection, script injection through stored data (brand, color or product names, notes, image links), or anything that reveals or changes data in a way the interface doesn't offer.
- **The Docker image:** a known vulnerability, with a fix available, in a package in the current `latest` image. The images are rebuilt every week and scanned with [Trivy](https://trivy.dev) before they're published, and again midweek, so check that the fix has been out for more than a few days.
- **The repository's workflows:** anything that could expose its secrets, or let someone else's code run with its permissions.

Out of scope:

- vulnerabilities in PostgreSQL, Rust, Node.js or the base images themselves; please report those to their own projects;
- anything that needs an attacker who can already change the server's configuration or database;
- the lack of user accounts, which is a known design choice (see above).
