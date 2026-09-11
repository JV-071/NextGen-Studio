# Third-party components

NextGen Studio source is original work under the MIT license. This license does
not relicense dependencies, reference projects, or assets loaded from a user's client.

The desktop application dynamically links Qt 6 Core, Gui and Widgets.
Qt is copyright The Qt Company Ltd. and other contributors. These modules are
available under LGPL v3 or commercial terms. See https://www.qt.io/licensing/
and https://code.qt.io/cgit/qt/qtbase.git/tree/LICENSES?h=v6.8.3 .
Distribution packages include Qt license texts in licenses/Qt. Corresponding
Qt source: https://download.qt.io/archive/qt/6.8/6.8.3/submodules/qtbase-everywhere-src-6.8.3.tar.xz .
Qt DLLs/shared libraries are separate and replaceable; no static Qt linking is used.
The editor source and build instructions are provided to allow rebuilding and
relinking against a compatible modified Qt. Qt's own third-party notices are
copied from the SDK when available.

No code or game artwork was copied from Assets Editor, Canary Studio, Honey,
NexaMap, Remere's Map Editor, Redux, or OTClientV8 Offline Map Explorer.
Reference-project ideas are documented in docs/reference-audit.md.
Build tools/actions are not linked into the application.

Versioned dependency notices and attribution metadata are bundled in licenses/Qt.
The inventory records exact upstream source URLs and SHA-256 hashes. These include
notices for Qt source components that may not all be enabled in a given binary.
ICU 73 runtime notices are included in licenses/ICU. Refresh using tools/update_notices.py.
