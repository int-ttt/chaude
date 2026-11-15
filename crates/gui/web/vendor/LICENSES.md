# Vendored libraries

The files in this folder are copied unchanged from the upstream npm/CDN builds and are embedded into the GUI page
(crates/gui/src/page.rs). Each file keeps its own license banner.

| File | Library | Version | License | Copyright |
|------|---------|---------|---------|-----------|
| marked.min.js | marked | 15.0.12 | MIT | Copyright (c) 2011-2025, Christopher Jeffrey |
| purify.min.js | DOMPurify | 3.2.6 | Apache-2.0 OR MPL-2.0 (dual; chaude takes Apache-2.0) | Copyright (c) Cure53 and other contributors |
| highlight.min.js | highlight.js | 11.11.1 | BSD-3-Clause | Copyright (c) 2006-2024, Josh Goebel and other contributors |

## marked (MIT)

Copyright (c) 2011-2025, Christopher Jeffrey

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the Software without restriction, including without limitation the
rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit
persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the
Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE
WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Source: https://github.com/markedjs/marked (tag v15.0.12)

## DOMPurify (Apache-2.0 OR MPL-2.0)

Copyright (c) Cure53 and other contributors

The banner in purify.min.js reads: "Released under the Apache license 2.0 and Mozilla Public License 2.0". chaude uses
it under the Apache License, Version 2.0, whose full text is at https://www.apache.org/licenses/LICENSE-2.0.txt. The
Mozilla Public License 2.0 text is at https://www.mozilla.org/MPL/2.0/. The upstream LICENSE file is in the DOMPurify
3.2.6 release.

Source: https://github.com/cure53/DOMPurify (tag 3.2.6)

## highlight.js (BSD-3-Clause)

Copyright (c) 2006-2024, Josh Goebel and other contributors

Redistribution and use in source and binary forms, with or without modification, are permitted provided that the
following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following
   disclaimer.
2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following
   disclaimer in the documentation and/or other materials provided with the distribution.
3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products
   derived from this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES,
INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

Source: https://github.com/highlightjs/highlight.js (tag 11.11.1)
