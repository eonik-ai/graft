COMMENT =	Change the hook. Keep the body.

DISTNAME =	graft-0.2.2

CATEGORIES =	multimedia

HOMEPAGE =	https://github.com/eonik-ai/graft

MAINTAINER =	eonik <connect@eonik.ai>

# Apache 2.0
PERMIT_PACKAGE =	Yes

WANTLIB +=	c pthread

BUILD_DEPENDS =	lang/rust
RUN_DEPENDS =	graphics/ffmpeg

SITES =		https://github.com/eonik-ai/graft/archive/refs/tags/
DISTFILES =	v0.2.2.tar.gz

WRKDIST =	${WRKDIR}/graft-0.2.2

do-build:
	cd ${WRKSRC} && cargo build --release --locked --bin graft

do-install:
	${INSTALL_PROGRAM} ${WRKSRC}/target/release/graft ${PREFIX}/bin/

.include <bsd.port.mk>
