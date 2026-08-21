# ob-pub

When an existing post's generated front matter or body changes, `ob-pub`
automatically writes an `updated` timestamp in `Asia/Shanghai`. A post's
original `date` is preserved, and repeated syncs with no content changes do not
change `updated`. New posts omit `updated`, allowing the site generator to use
the publication date as the initial update time.
