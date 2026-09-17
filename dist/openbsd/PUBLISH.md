# OpenBSD ports

Needs a ports CVS account (see https://www.openbsd.org/faq/faq15.html).

Until that login exists, the recipe is ready in this directory:

- [`Makefile`](Makefile)
- [`distinfo`](distinfo) — SHA256/SIZE of
  `https://github.com/eonik-ai/graft/archive/refs/tags/v0.2.2.tar.gz`

Place as `multimedia/graft/` in the ports tree, then:

```sh
cd multimedia/graft
make makesum   # if distinfo needs a refresh
make
make install
```

Send the diff to `ports@openbsd.org` once you have a CVS login, or ask
a ports committer to import it.
