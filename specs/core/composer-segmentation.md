# Composer Segmentation

The composer owns roman chunk segmentation before decoder ranking.

When multiple exact segmentations cover the same normalized input with the same
number of chunks, prefer the path with fewer weak chunks before applying larger
chunk-shape tie-breaks. A weak chunk is a 1- or 2-character exact chunk unless
that exact chunk is a very high-frequency Khmer word.

This keeps common Khmer phrase chunks such as `mean|nek|bong|tte` from being
split through shorter cross-source entries such as `me|anne|kbong|tte`.
It still allows common short phrase anchors such as `ge|tteng|os` for
`gettengos`.
