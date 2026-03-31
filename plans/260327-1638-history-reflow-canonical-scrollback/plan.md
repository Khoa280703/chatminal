Diagnose performance issues
79
Performance
85
Accessibility
100
Best Practices
92
SEO
79
FCP
+5
LCP
+13
TBT
+30
CLS
+25
SI
+7
Performance
Values are estimated and may vary. The performance score is calculated directly from these metrics.See calculator.
0–49
50–89
90–100
Final Screenshot

METRICS
Expand view
First Contentful Paint
3.2 s
Largest Contentful Paint
3.9 s
Total Blocking Time
0 ms
Cumulative Layout Shift
0
Speed Index
4.6 s
Captured at Mar 30, 2026 at 8:28 AM GMT+7
Emulated Moto G Power with Lighthouse 13.0.1
Single page session
Initial page load
Slow 4G throttling
Using HeadlessChromium 146.0.7680.153 with lr
View Treemap
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Show audits relevant to:

All

FCP

LCP

TBT

CLS
INSIGHTS
Render blocking requests Est savings of 610 ms
Requests are blocking the page's initial render, which may delay LCP. Deferring or inlining can move these network requests out of the critical path.LCPFCPUnscored
  Show 3rd-party resources (1)
URL
Transfer Size
Duration
snapvie.com 1st Party
19.3 KiB	2,520 ms
…assets/FormatPicker.BPjxvjyR.css(snapvie.com)
4.3 KiB
630 ms
…assets/3.Ds_1vsX4.css(snapvie.com)
2.5 KiB
470 ms
…assets/knowledge-sections.ENbp9D_B.css(snapvie.com)
1.5 KiB
470 ms
…assets/0.BvN9DDzT.css(snapvie.com)
10.0 KiB
790 ms
…assets/AppIcon.BlEdAg33.css(snapvie.com)
1.0 KiB
160 ms
Google Fonts Cdn 
1.1 KiB	750 ms
/css2?family=…(fonts.googleapis.com)
1.1 KiB
750 ms
Forced reflow
A forced reflow occurs when JavaScript queries geometric properties (such as offsetWidth) after styles have been invalidated by a change to the DOM state. This can result in poor performance. Learn more about forced reflows and possible mitigations.Unscored
Source
Total reflow time
[unattributed]
22 ms
…chunks/DdlfWAGz.js:1:5564(snapvie.com)
34 ms
Network dependency tree
Avoid chaining critical requests by reducing the length of chains, reducing the download size of resources, or deferring the download of unnecessary resources to improve page load.LCPUnscored
Maximum critical path latency: 546 ms
Initial Navigation
https://snapvie.com - 218 ms, 19.11 KiB
/css2?family=…(fonts.googleapis.com) - 221 ms, 1.10 KiB
…assets/AppIcon.BlEdAg33.css(snapvie.com) - 265 ms, 0.97 KiB
…assets/0.BvN9DDzT.css(snapvie.com) - 267 ms, 10.02 KiB
…assets/fredoka-latin.DM6njrJ3.woff2(snapvie.com) - 546 ms, 29.78 KiB
…assets/nunito-no….BzFMHfZw.woff2(snapvie.com) - 545 ms, 38.96 KiB
…assets/knowledge-sections.ENbp9D_B.css(snapvie.com) - 275 ms, 1.50 KiB
…assets/FormatPicker.BPjxvjyR.css(snapvie.com) - 378 ms, 4.31 KiB
…assets/3.Ds_1vsX4.css(snapvie.com) - 276 ms, 2.47 KiB
Preconnected origins
preconnect hints help the browser establish a connection earlier in the page load, saving time when the first request for that origin is made. The following are the origins that the page preconnected to.
Origin
Source
https://fonts.googleapis.com/
head > link
<link rel="preconnect" href="https://fonts.googleapis.com">
https://fonts.gstatic.com/
head > link
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
Unused preconnect. Only use preconnect for origins that the page is likely to request.
Preconnect candidates
Add preconnect hints to your most important origins, but try to use no more than 4.
Origin
Est LCP savings
https://cloudflareinsights.com
300 ms
Use efficient cache lifetimes Est savings of 4 KiB
A long cache lifetime can speed up repeat visits to your page. Learn more about caching.LCPFCPUnscored
  Show 3rd-party resources (1)
Request
Cache TTL
Transfer Size
Cloudflare Utility 
11 KiB
/beacon.min.js(static.cloudflareinsights.com)
1d
11 KiB
snapvie.com 1st Party
1 KiB
/logo.svg(snapvie.com)
7d
1 KiB
LCP breakdown
Each subpart has specific improvement strategies. Ideally, most of the LCP time should be spent on loading the resources, not within delays.LCPUnscored
Subpart
Duration
Time to first byte
0 ms
Element render delay
2,360 ms
Download YouTube Videos in 4K and 8K HDR.
<h1 class="text-4xl md:text-6xl lg:text-7xl font-bold text-plum mb-4 leading-[0.95] t…">
3rd parties
3rd party code can significantly impact load performance. Reduce and defer loading of 3rd party code to prioritize your page's content.Unscored
3rd party
Transfer size
Main thread time
Cloudflare Utility 
11 KiB	8 ms
/beacon.min.js(static.cloudflareinsights.com)
11 KiB
8 ms
cloudflareinsights.com
0 KiB	0 ms
/cdn-cgi/rum(cloudflareinsights.com)
0 KiB
0 ms
Google Fonts Cdn 
1 KiB	0 ms
/css2?family=…(fonts.googleapis.com)
1 KiB
0 ms
These insights are also available in the Chrome DevTools Performance Panel - record a trace to view more detailed information.
DIAGNOSTICS
Reduce unused JavaScript Est savings of 90 KiB
Reduce unused JavaScript and defer loading scripts until they are required to decrease bytes consumed by network activity. Learn how to reduce unused JavaScript.LCPFCPUnscored
URL
Transfer Size
Est Savings
snapvie.com 1st Party
122.4 KiB	89.5 KiB
…nodes/3.-N0kgPYX.js(snapvie.com)
71.4 KiB
51.0 KiB
…chunks/Dz4HgEz8.js(snapvie.com)
51.0 KiB
38.5 KiB
Image elements do not have explicit width and height
Set an explicit width and height on image elements to reduce layout shifts and improve CLS. Learn how to set image dimensionsCLSUnscored
URL
snapvie.com 1st Party
Snapvie
<img src="/logo.svg" alt="Snapvie" class="h-10 w-auto shrink-0 drop-shadow-sm transition-transform duration-300 grou…" loading="eager" decoding="async">
/logo.svg(snapvie.com)
Minify JavaScript Est savings of 5 KiB
Minifying JavaScript files can reduce payload sizes and script parse time. Learn how to minify JavaScript.LCPFCPUnscored
URL
Transfer Size
Est Savings
snapvie.com 1st Party
6.1 KiB	5.1 KiB
…chunks/iRUoMHWx.js(snapvie.com)
6.1 KiB
5.1 KiB
Avoid long main-thread tasks 1 long task found
Lists the longest tasks on the main thread, useful for identifying worst contributors to input delay. Learn how to avoid long main-thread tasksTBTUnscored
URL
Start Time
Duration
snapvie.com 1st Party
73 ms
…chunks/nnO7g7f2.js(snapvie.com)
3,001 ms
73 ms
More information about the performance of your application. These numbers don't directly affect the Performance score.
PASSED AUDITS (17)
Show
85
Accessibility
These checks highlight opportunities to improve the accessibility of your web app. Automatic detection can only detect a subset of issues and does not guarantee the accessibility of your web app, so manual testing is also encouraged.
ARIA
Elements with an ARIA [role] that require children to contain a specific [role] are missing some or all of those required children.
Some ARIA parent roles must contain specific child roles to perform their intended accessibility functions. Learn more about roles and required children elements.
Failing Elements
Single Playlist
<div class="playlist-mode-switch" role="tablist" aria-label="Download mode">
Single
<button type="button" class="playlist-mode-option playlist-mode-option-active" aria-pressed="true">
Playlist
<button type="button" class="playlist-mode-option " aria-pressed="false">
These are opportunities to improve the usage of ARIA in your application which may enhance the experience for users of assistive technology, like a screen reader.
NAMES AND LABELS
Buttons do not have an accessible name
When a button doesn't have an accessible name, screen readers announce it as "button", making it unusable for users who rely on screen readers. Learn how to make buttons more accessible.
Failing Elements
header.glass-header > div.max-w-7xl > div.md:hidden > button.text-plum
<button type="button" class="text-plum p-2 rounded-xl hover:bg-white/50 transition-colors flex items-ce…">
div.mx-auto > form#download-options > div.relative > button.absolute
<button class="absolute right-1.5 top-1.5 bottom-1.5 flex items-center justify-center rou…" type="submit">
Image elements do not have [alt] attributes that are redundant text.
Informative elements should aim for short, descriptive alternative text. Alternative text that is exactly the same as the text adjacent to the link or image is potentially confusing for screen reader users, because the text will be read twice. Learn more about the alt attribute.Unscored
Failing Elements
Snapvie
<img src="/logo.svg" alt="Snapvie" class="h-10 w-auto shrink-0 drop-shadow-sm transition-transform duration-300 grou…" loading="eager" decoding="async">
These are opportunities to improve the semantics of the controls in your application. This may enhance the experience for users of assistive technology, like a screen reader.
CONTRAST
Background and foreground colors do not have a sufficient contrast ratio.
Low-contrast text is difficult or impossible for many users to read. Learn how to provide sufficient color contrast.
Failing Elements
GUIDES
<p class="resource-group-label svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
View all guides →
<a href="/guides" class="resource-cta svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
COMPARE
<p class="resource-group-label svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
View all comparisons →
<a href="/compare" class="resource-cta svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
These are opportunities to improve the legibility of your content.
NAVIGATION
Heading elements are not in a sequentially-descending order
Properly ordered headings that do not skip levels convey the semantic structure of the page, making it easier to navigate and understand when using assistive technologies. Learn more about heading order.
Failing Elements
1. Find Video
<h3 class="how-it-works-step-title text-xl font-bold text-plum mb-2 svelte-j0od2k">
These are opportunities to improve keyboard navigation in your application.
ADDITIONAL ITEMS TO MANUALLY CHECK (10)
Hide
Interactive controls are keyboard focusable
Interactive elements indicate their purpose and state
The page has a logical tab order
Visual order on the page follows DOM order
User focus is not accidentally trapped in a region
The user's focus is directed to new content added to the page
HTML5 landmark elements are used to improve navigation
Offscreen content is hidden from assistive technology
Custom controls have associated labels
Custom controls have ARIA roles
These items address areas which an automated testing tool cannot cover. Learn more in our guide on conducting an accessibility review.
PASSED AUDITS (22)
Hide
[aria-*] attributes match their roles
[aria-hidden="true"] is not present on the document <body>
[role]s have all required [aria-*] attributes
[role] values are valid
[aria-*] attributes have valid values
[aria-*] attributes are valid and not misspelled
Image elements have [alt] attributes
Form elements have associated labels
[user-scalable="no"] is not used in the <meta name="viewport"> element and the [maximum-scale] attribute is not less than 5.
ARIA attributes are used as specified for the element's role
[aria-hidden="true"] elements do not contain focusable descendents
Elements use only permitted ARIA attributes
Document has a <title> element
<html> element has a [lang] attribute
<html> element has a valid value for its [lang] attribute
Links are distinguishable without relying on color.
Links have a discernible name
Lists contain only <li> elements and script supporting elements (<script> and <template>).
List items (<li>) are contained within <ul>, <ol> or <menu> parent elements
Touch targets have sufficient size and spacing.
Document has a main landmark.
Deprecated ARIA roles were not used
NOT APPLICABLE (33)
Hide
[accesskey] values are unique
button, link, and menuitem elements have accessible names
Elements with role="dialog" or role="alertdialog" have accessible names.
ARIA input fields have accessible names
ARIA meter elements have accessible names
ARIA progressbar elements have accessible names
[role]s are contained by their required parent element
Elements with the role=text attribute do not have focusable descendents.
ARIA toggle fields have accessible names
ARIA tooltip elements have accessible names
ARIA treeitem elements have accessible names
The page contains a heading, skip link, or landmark region
<dl>'s contain only properly-ordered <dt> and <dd> groups, <script>, <template> or <div> elements.
Definition list items are wrapped in <dl> elements
ARIA IDs are unique
No form fields have multiple labels
<frame> or <iframe> elements have a title
<html> element has an [xml:lang] attribute with the same base language as the [lang] attribute.
Input buttons have discernible text.
<input type="image"> elements have [alt] text
The document does not use <meta http-equiv="refresh">
<object> elements have alternate text
Select elements have associated label elements.
Skip links are focusable.
No element has a [tabindex] value greater than 0
Cells in a <table> element that use the [headers] attribute refer to table cells within the same table.
<th> elements and elements with [role="columnheader"/"rowheader"] have data cells they describe.
[lang] attributes have a valid value
<video> elements contain a <track> element with [kind="captions"]
Tables have different content in the summary attribute and <caption>.
All heading elements contain content.
Uses ARIA roles only on compatible elements
Identical links have the same purpose.
100
Best Practices
TRUST AND SAFETY
Ensure CSP is effective against XSS attacks
Use a strong HSTS policy
Ensure proper origin isolation with COOP
Mitigate clickjacking with XFO or CSP
Mitigate DOM-based XSS with Trusted Types
PASSED AUDITS (13)
Show
NOT APPLICABLE (2)
Show
92
SEO
These checks ensure that your page is following basic search engine optimization advice. There are many additional factors Lighthouse does not score here that may affect your search ranking, including performance on Core Web Vitals. Learn more about Google Search Essentials.
CRAWLING AND INDEXING
robots.txt is not valid 2 errors found
If your robots.txt file is malformed, crawlers may not be able to understand how you want your website to be crawled or indexed. Learn more about robots.txt.
Line #
Content
Error
29
Content-Signal: search=yes,ai-train=no
Unknown directive
68
Llms-Txt: https://snapvie.com/llms.txt
Unknown directive
To appear in search results, crawlers need access to your app.
ADDITIONAL ITEMS TO MANUALLY CHECK (1)
Hide
Structured data is valid
Run these additional validators on your site to check additional SEO best practices.
PASSED AUDITS (9)
Hide
Page isn’t blocked from indexing
Document has a <title> element
Document has a meta description
Page has successful HTTP status code
Links have descriptive text
Links are crawlable
Image elements have [alt] attributes
Document has a valid hreflang
Document has a valid rel=canonical

Discover what your real users are experiencing
No Data

Diagnose performance issues
100
Performance
90
Accessibility
100
Best Practices
92
SEO
100
FCP
+10
LCP
+25
TBT
+30
CLS
+25
SI
+10
Performance
Values are estimated and may vary. The performance score is calculated directly from these metrics.See calculator.
0–49
50–89
90–100
Final Screenshot

METRICS
Expand view
First Contentful Paint
0.7 s
Largest Contentful Paint
0.7 s
Total Blocking Time
0 ms
Cumulative Layout Shift
0.036
Speed Index
0.7 s
Captured at Mar 30, 2026 at 8:28 AM GMT+7
Emulated Desktop with Lighthouse 13.0.1
Single page session
Initial page load
Custom throttling
Using HeadlessChromium 146.0.7680.153 with lr
View Treemap
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Screenshot
Show audits relevant to:

All

FCP

LCP

TBT

CLS
INSIGHTS
Render blocking requests Est savings of 120 ms
Requests are blocking the page's initial render, which may delay LCP. Deferring or inlining can move these network requests out of the critical path.LCPFCPUnscored
  Show 3rd-party resources (1)
URL
Transfer Size
Duration
snapvie.com 1st Party
19.3 KiB	600 ms
…assets/AppIcon.BlEdAg33.css(snapvie.com)
1.0 KiB
…assets/FormatPicker.BPjxvjyR.css(snapvie.com)
4.3 KiB
140 ms
…assets/knowledge-sections.ENbp9D_B.css(snapvie.com)
1.5 KiB
140 ms
…assets/0.BvN9DDzT.css(snapvie.com)
10.0 KiB
190 ms
…assets/3.Ds_1vsX4.css(snapvie.com)
2.5 KiB
140 ms
Google Fonts Cdn 
1.1 KiB	230 ms
/css2?family=…(fonts.googleapis.com)
1.1 KiB
230 ms
Forced reflow
A forced reflow occurs when JavaScript queries geometric properties (such as offsetWidth) after styles have been invalidated by a change to the DOM state. This can result in poor performance. Learn more about forced reflows and possible mitigations.Unscored
Source
Total reflow time
[unattributed]
29 ms
…chunks/DdlfWAGz.js:1:5564(snapvie.com)
8 ms
Network dependency tree
Avoid chaining critical requests by reducing the length of chains, reducing the download size of resources, or deferring the download of unnecessary resources to improve page load.LCPUnscored
Maximum critical path latency: 360 ms
Initial Navigation
https://snapvie.com - 132 ms, 19.11 KiB
/css2?family=…(fonts.googleapis.com) - 125 ms, 1.10 KiB
…assets/AppIcon.BlEdAg33.css(snapvie.com) - 288 ms, 0.97 KiB
…assets/0.BvN9DDzT.css(snapvie.com) - 132 ms, 10.03 KiB
…assets/fredoka-latin.DM6njrJ3.woff2(snapvie.com) - 360 ms, 29.80 KiB
…assets/nunito-no….BzFMHfZw.woff2(snapvie.com) - 359 ms, 38.98 KiB
…assets/knowledge-sections.ENbp9D_B.css(snapvie.com) - 170 ms, 1.51 KiB
…assets/FormatPicker.BPjxvjyR.css(snapvie.com) - 201 ms, 4.30 KiB
…assets/3.Ds_1vsX4.css(snapvie.com) - 127 ms, 2.50 KiB
Preconnected origins
preconnect hints help the browser establish a connection earlier in the page load, saving time when the first request for that origin is made. The following are the origins that the page preconnected to.
Origin
Source
https://fonts.googleapis.com/
head > link
<link rel="preconnect" href="https://fonts.googleapis.com">
https://fonts.gstatic.com/
head > link
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous">
Unused preconnect. Only use preconnect for origins that the page is likely to request.
Preconnect candidates
Add preconnect hints to your most important origins, but try to use no more than 4.
No additional origins are good candidates for preconnecting
Use efficient cache lifetimes Est savings of 4 KiB
A long cache lifetime can speed up repeat visits to your page. Learn more about caching.LCPFCPUnscored
  Show 3rd-party resources (1)
Request
Cache TTL
Transfer Size
Cloudflare Utility 
11 KiB
/beacon.min.js(static.cloudflareinsights.com)
1d
11 KiB
snapvie.com 1st Party
1 KiB
/logo.svg(snapvie.com)
7d
1 KiB
Layout shift culprits
Layout shifts occur when elements move absent any user interaction. Investigate the causes of layout shifts, such as elements being added, removed, or their fonts changing as the page loads.CLSUnscored
Element
Layout shift score
Total
0.036
1. Find Video Browse your favorite sites like YouTube or TikTok and copy the U…
<section class="how-it-works-section defer-render-how-it-works py-8 px-6 lg:px-20 relative…" id="how-it-works">
0.036
Home How it Works Guides Compare EN LOGIN
<div class="hidden md:flex items-center gap-8">
0.000
Optimize DOM size
A large DOM can increase the duration of style calculations and layout reflows, impacting page responsiveness. A large DOM will also increase memory usage. Learn how to avoid an excessive DOM size.Unscored
Statistic
Element
Value
Total elements
324
DOM depth
div.why-snapvie-feature-head > div.why-snapvie-feature-icon > svg.lucide-icon > path
<path d="M21 5H3">
15
Most children
div.hidden > div.relative > button.language-switcher-trigger > svg.lucide-icon
<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide-icon lucide lucide-languages app-icon text-base" aria-hidden="true">
6
LCP breakdown
Each subpart has specific improvement strategies. Ideally, most of the LCP time should be spent on loading the resources, not within delays.LCPUnscored
Subpart
Duration
Time to first byte
0 ms
Element render delay
410 ms
Download YouTube Videos in 4K and 8K HDR.
<h1 class="text-4xl md:text-6xl lg:text-7xl font-bold text-plum mb-4 leading-[0.95] t…">
3rd parties
3rd party code can significantly impact load performance. Reduce and defer loading of 3rd party code to prioritize your page's content.Unscored
3rd party
Transfer size
Main thread time
Cloudflare Utility 
11 KiB	11 ms
/beacon.min.js(static.cloudflareinsights.com)
11 KiB
11 ms
cloudflareinsights.com
0 KiB	0 ms
/cdn-cgi/rum(cloudflareinsights.com)
0 KiB
0 ms
Google Fonts Cdn 
1 KiB	0 ms
/css2?family=…(fonts.googleapis.com)
1 KiB
0 ms
These insights are also available in the Chrome DevTools Performance Panel - record a trace to view more detailed information.
DIAGNOSTICS
Reduce unused JavaScript Est savings of 90 KiB
Reduce unused JavaScript and defer loading scripts until they are required to decrease bytes consumed by network activity. Learn how to reduce unused JavaScript.LCPFCPUnscored
URL
Transfer Size
Est Savings
snapvie.com 1st Party
122.4 KiB	89.5 KiB
…nodes/3.-N0kgPYX.js(snapvie.com)
71.4 KiB
51.0 KiB
…chunks/Dz4HgEz8.js(snapvie.com)
51.0 KiB
38.5 KiB
Minify JavaScript Est savings of 5 KiB
Minifying JavaScript files can reduce payload sizes and script parse time. Learn how to minify JavaScript.LCPFCPUnscored
URL
Transfer Size
Est Savings
snapvie.com 1st Party
6.1 KiB	5.1 KiB
…chunks/iRUoMHWx.js(snapvie.com)
6.1 KiB
5.1 KiB
Image elements do not have explicit width and height
Set an explicit width and height on image elements to reduce layout shifts and improve CLS. Learn how to set image dimensionsCLSUnscored
URL
snapvie.com 1st Party
Snapvie
<img src="/logo.svg" alt="Snapvie" class="h-10 w-auto shrink-0 drop-shadow-sm transition-transform duration-300 grou…" loading="eager" decoding="async">
/logo.svg(snapvie.com)
Avoid long main-thread tasks 1 long task found
Lists the longest tasks on the main thread, useful for identifying worst contributors to input delay. Learn how to avoid long main-thread tasksTBTUnscored
URL
Start Time
Duration
snapvie.com 1st Party
53 ms
…chunks/nnO7g7f2.js(snapvie.com)
681 ms
53 ms
More information about the performance of your application. These numbers don't directly affect the Performance score.
PASSED AUDITS (15)
Show
90
Accessibility
These checks highlight opportunities to improve the accessibility of your web app. Automatic detection can only detect a subset of issues and does not guarantee the accessibility of your web app, so manual testing is also encouraged.
ARIA
Elements with an ARIA [role] that require children to contain a specific [role] are missing some or all of those required children.
Some ARIA parent roles must contain specific child roles to perform their intended accessibility functions. Learn more about roles and required children elements.
Failing Elements
Single Playlist
<div class="playlist-mode-switch" role="tablist" aria-label="Download mode">
Single
<button type="button" class="playlist-mode-option playlist-mode-option-active" aria-pressed="true">
Playlist
<button type="button" class="playlist-mode-option " aria-pressed="false">
These are opportunities to improve the usage of ARIA in your application which may enhance the experience for users of assistive technology, like a screen reader.
CONTRAST
Background and foreground colors do not have a sufficient contrast ratio.
Low-contrast text is difficult or impossible for many users to read. Learn how to provide sufficient color contrast.
Failing Elements
GUIDES
<p class="resource-group-label svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
View all guides →
<a href="/guides" class="resource-cta svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
COMPARE
<p class="resource-group-label svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
View all comparisons →
<a href="/compare" class="resource-cta svelte-bmxau1">
Guides & Resources GUIDES How to Use Snapvie How to Download Playlists Why Do…
<div class="knowledge-card svelte-bmxau1">
Frequently Asked Questions Can Snapvie download YouTube videos in 8K HDR? ▼ Why…
<section class="knowledge-section py-10 px-6 lg:px-20 bg-white border-t border-pink-50 sve…">
These are opportunities to improve the legibility of your content.
NAVIGATION
Heading elements are not in a sequentially-descending order
Properly ordered headings that do not skip levels convey the semantic structure of the page, making it easier to navigate and understand when using assistive technologies. Learn more about heading order.
Failing Elements
1. Find Video
<h3 class="how-it-works-step-title text-xl font-bold text-plum mb-2 svelte-j0od2k">
These are opportunities to improve keyboard navigation in your application.
NAMES AND LABELS
Image elements do not have [alt] attributes that are redundant text.
Informative elements should aim for short, descriptive alternative text. Alternative text that is exactly the same as the text adjacent to the link or image is potentially confusing for screen reader users, because the text will be read twice. Learn more about the alt attribute.Unscored
Failing Elements
Snapvie
<img src="/logo.svg" alt="Snapvie" class="h-10 w-auto shrink-0 drop-shadow-sm transition-transform duration-300 grou…" loading="eager" decoding="async">
These are opportunities to improve the semantics of the controls in your application. This may enhance the experience for users of assistive technology, like a screen reader.
ADDITIONAL ITEMS TO MANUALLY CHECK (10)
Hide
Interactive controls are keyboard focusable
Interactive elements indicate their purpose and state
The page has a logical tab order
Visual order on the page follows DOM order
User focus is not accidentally trapped in a region
The user's focus is directed to new content added to the page
HTML5 landmark elements are used to improve navigation
Offscreen content is hidden from assistive technology
Custom controls have associated labels
Custom controls have ARIA roles
These items address areas which an automated testing tool cannot cover. Learn more in our guide on conducting an accessibility review.
PASSED AUDITS (23)
Hide
[aria-*] attributes match their roles
[aria-hidden="true"] is not present on the document <body>
[role]s have all required [aria-*] attributes
[role] values are valid
[aria-*] attributes have valid values
[aria-*] attributes are valid and not misspelled
Buttons have an accessible name
Image elements have [alt] attributes
Form elements have associated labels
[user-scalable="no"] is not used in the <meta name="viewport"> element and the [maximum-scale] attribute is not less than 5.
ARIA attributes are used as specified for the element's role
[aria-hidden="true"] elements do not contain focusable descendents
Elements use only permitted ARIA attributes
Document has a <title> element
<html> element has a [lang] attribute
<html> element has a valid value for its [lang] attribute
Links are distinguishable without relying on color.
Links have a discernible name
Lists contain only <li> elements and script supporting elements (<script> and <template>).
List items (<li>) are contained within <ul>, <ol> or <menu> parent elements
Touch targets have sufficient size and spacing.
Document has a main landmark.
Deprecated ARIA roles were not used
NOT APPLICABLE (33)
Hide
[accesskey] values are unique
button, link, and menuitem elements have accessible names
Elements with role="dialog" or role="alertdialog" have accessible names.
ARIA input fields have accessible names
ARIA meter elements have accessible names
ARIA progressbar elements have accessible names
[role]s are contained by their required parent element
Elements with the role=text attribute do not have focusable descendents.
ARIA toggle fields have accessible names
ARIA tooltip elements have accessible names
ARIA treeitem elements have accessible names
The page contains a heading, skip link, or landmark region
<dl>'s contain only properly-ordered <dt> and <dd> groups, <script>, <template> or <div> elements.
Definition list items are wrapped in <dl> elements
ARIA IDs are unique
No form fields have multiple labels
<frame> or <iframe> elements have a title
<html> element has an [xml:lang] attribute with the same base language as the [lang] attribute.
Input buttons have discernible text.
<input type="image"> elements have [alt] text
The document does not use <meta http-equiv="refresh">
<object> elements have alternate text
Select elements have associated label elements.
Skip links are focusable.
No element has a [tabindex] value greater than 0
Cells in a <table> element that use the [headers] attribute refer to table cells within the same table.
<th> elements and elements with [role="columnheader"/"rowheader"] have data cells they describe.
[lang] attributes have a valid value
<video> elements contain a <track> element with [kind="captions"]
Tables have different content in the summary attribute and <caption>.
All heading elements contain content.
Uses ARIA roles only on compatible elements
Identical links have the same purpose.
100
Best Practices
TRUST AND SAFETY
Ensure CSP is effective against XSS attacks
Use a strong HSTS policy
Ensure proper origin isolation with COOP
Mitigate clickjacking with XFO or CSP
Mitigate DOM-based XSS with Trusted Types
PASSED AUDITS (13)
Show
NOT APPLICABLE (2)
Show
92
SEO
These checks ensure that your page is following basic search engine optimization advice. There are many additional factors Lighthouse does not score here that may affect your search ranking, including performance on Core Web Vitals. Learn more about Google Search Essentials.
CRAWLING AND INDEXING
robots.txt is not valid 2 errors found
If your robots.txt file is malformed, crawlers may not be able to understand how you want your website to be crawled or indexed. Learn more about robots.txt.
Line #
Content
Error
29
Content-Signal: search=yes,ai-train=no
Unknown directive
68
Llms-Txt: https://snapvie.com/llms.txt
Unknown directive
To appear in search results, crawlers need access to your app.
ADDITIONAL ITEMS TO MANUALLY CHECK (1)
Hide
Structured data is valid
Run these additional validators on your site to check additional SEO best practices.
PASSED AUDITS (9)
Hide
Page isn’t blocked from indexing
Document has a <title> element
Document has a meta description
Page has successful HTTP status code
Links have descriptive text
Links are crawlable
Image elements have [alt] attributes
Document has a valid hreflang
Document has a valid rel=canonical