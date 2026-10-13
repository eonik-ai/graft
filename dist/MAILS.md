# Send these three from Gmail today

From **must** be `abinashsena@gmail.com`.
Turn on **plain text** (compose → three dots → Plain text).
Do not send from Mail.app or local postfix — Debian BTS will bounce a
machine From.

For each mail: copy **To** and **Subject** into Gmail's fields. Paste
only the **Body** into the message. Do not wrap it in HTML.

These three mails are the remaining **email** path to 100% of pkg.bot
families that accept mail. After they land:

| Mail | File | Unblocks |
| --- | --- | --- |
| 1 Debian ITP | [`debian/MAIL.txt`](debian/MAIL.txt) | Debian + every debuntu clone |
| 2 OpenBSD NEW port | [`openbsd/MAIL.txt`](openbsd/MAIL.txt) | OpenBSD Ports (no CVS login needed to *send*) |
| 3 GURU patch | [`gentoo/MAIL.txt`](gentoo/MAIL.txt) | GURU + LiGurOS (PRs first, then access) |

Not mail (still required for 100%):

- Fedora Package Review is Bugzilla, not SMTP. Paste [`rpm/REVIEW-REQUEST.txt`](rpm/REVIEW-REQUEST.txt) after FAS: https://bugzilla.redhat.com/enter_bug.cgi?product=Fedora&component=Package%20Review
- AUR is SSH: [`aur/PUBLISH.md`](aur/PUBLISH.md)
- Alpine is GitLab: [`alpine/PUBLISH.md`](alpine/PUBLISH.md)

Reply with the Debian BTS `#nnnnnn` when mail 1 acks.
