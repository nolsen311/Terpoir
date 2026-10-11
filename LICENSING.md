# Licensing

Copyright (C) 2026 The Terpoir Authors

Terpoir is free software: you can redistribute it and/or modify it under the
terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

Terpoir is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
A PARTICULAR PURPOSE. See the GNU General Public License for more details.

The full license text is in [LICENSE](LICENSE).

## Additional permissions under GNU GPL version 3 section 7

The SPDX identifier `AdditionRef-Terpoir-AppStore-exception` in the source
file headers refers to both permissions in this section.

### Application stores

As an additional permission under section 7 of the GNU General Public
License, version 3, you may distribute this program, or any covered work,
through an application store or similar distribution channel (including, but
not limited to, the Apple App Store and Google Play), even if the terms and
conditions of that channel impose restrictions that would otherwise be
incompatible with the GNU General Public License, provided that the
Corresponding Source of the work, including your modifications, is also made
available under the GNU General Public License, with or without this
permission, through a channel that does not impose those restrictions.

If you modify this program, you may extend this additional permission to your
version, but you are not obligated to do so. If you do not wish to do so,
delete this additional permission from your version.

### Google Play libraries

If you modify this program, or any covered work, by linking or combining it
with the Google Play Libraries (or a modified version of those libraries),
containing parts covered by the terms of the Android Software Development
Kit License or other terms under which Google distributes them, the
licensors of this program grant you additional permission to convey the
resulting work. Corresponding Source for a non-source form of such a
combination shall not include the source code for the parts of the Google
Play Libraries used, but shall include the Corresponding Source of the
covered work.

"Google Play Libraries" means the client libraries that Google publishes
for apps to use Google Play and Google Play services, together with the
libraries Google distributes with them as their dependencies; for example
those in the Maven groups `com.google.android.play`,
`com.google.android.gms` and `com.android.billingclient`. This permission
does not extend to any other library that is not free software.

Terpoir uses these libraries only in its `play` build flavor. The `foss`
flavor contains none of them, and its build fails if one appears.

If you modify this program, you may extend this additional permission to your
version, but you are not obligated to do so. If you do not wish to do so,
delete this additional permission from your version.

## Trademarks

The GPL grants rights to the code, not to the Terpoir name or logo. GPLv3
section 7(e) permits declining to grant trademark rights, and we do. See
[TRADEMARKS.md](TRADEMARKS.md).

## Contributions

Contributions are accepted under the same terms, including the additional
permissions above, and must carry a Developer Certificate of Origin sign-off.
See [CONTRIBUTING.md](CONTRIBUTING.md).

## Source file header

New source files should begin with:

```
// SPDX-License-Identifier: GPL-3.0-or-later WITH AdditionRef-Terpoir-AppStore-exception
// Copyright (C) 2026 The Terpoir Authors
// See LICENSING.md for the additional permission under GPLv3 section 7.
```

(The header's wording predates the second permission; the identifier covers
both, as stated above.)
