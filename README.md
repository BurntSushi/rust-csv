A comparison of different sort algorithms in Rust. This includes mergesort, 
quicksort, heapsort, insertion sort, selection sort, bubble sort and even bogo 
sort. The library comes with benchmarks for vectors of different sizes and for 
vectors that are already sorted or vectors where all elements are equivalent.
This library also comes with QuickCheck tests that check whether the result of 
a sorting algorithm is sorted. Each algorithm is also checked that it is 
stable/unstable depending on the algorithm used.

There is some documentation of the API: http://burntsushi.net/rustdoc/sorts

Many of the implementations were done with inspiration from the relevant
Wikipedia articles.

Tests can be run with `make test`. Benchmarks can be run with `make bench`. 
Since they can take a long time to run, here are all benchmarks on my machine.
My specs: Intel i3930K (12 threads) with 32GB of memory. Compiled with `-O`.

Note that sorting algorithms with average case `O(n^2)` or worse complexity are 
not benchmarked on the `medium` and `large` sizes.

The `std` sort is the one from Rust's standard library.

```
micro_bubble                      119 ns/iter (+/- 22)
micro_heapsort_down               169 ns/iter (+/- 16)
micro_heapsort_up                 173 ns/iter (+/- 13)
micro_insertion                   124 ns/iter (+/- 33)
micro_mergesort                   282 ns/iter (+/- 7)
micro_mergesort_insertion         184 ns/iter (+/- 40)
micro_quicksort_dumb              235 ns/iter (+/- 63)
micro_quicksort_insertion         129 ns/iter (+/- 21)
micro_quicksort_smart             234 ns/iter (+/- 31)
micro_selection                   133 ns/iter (+/- 9)
micro_std                         121 ns/iter (+/- 36)
small_bubble                    11306 ns/iter (+/- 1622)
small_heapsort_down              2782 ns/iter (+/- 101)
small_heapsort_up                2790 ns/iter (+/- 99)
small_insertion                  6396 ns/iter (+/- 964)
small_mergesort                  3263 ns/iter (+/- 99)
small_mergesort_insertion        2521 ns/iter (+/- 192)
small_quicksort_dumb             2679 ns/iter (+/- 264)
small_quicksort_insertion        2454 ns/iter (+/- 263)
small_quicksort_smart            2627 ns/iter (+/- 213)
small_selection                  6436 ns/iter (+/- 354)
small_std                        2370 ns/iter (+/- 198)
medium_heapsort_down           664567 ns/iter (+/- 3372)
medium_heapsort_up             708254 ns/iter (+/- 2642)
medium_mergesort               982694 ns/iter (+/- 3843)
medium_mergesort_insertion     886573 ns/iter (+/- 4352)
medium_quicksort_dumb          686721 ns/iter (+/- 24446)
medium_quicksort_insertion     678892 ns/iter (+/- 15685)
medium_quicksort_smart         690518 ns/iter (+/- 18175)
medium_std                     494699 ns/iter (+/- 1971)
large_heapsort_down          10174136 ns/iter (+/- 187066)
large_heapsort_up            10659817 ns/iter (+/- 134254)
large_mergesort              12287202 ns/iter (+/- 53687)
large_mergesort_insertion    11349007 ns/iter (+/- 37731)
large_quicksort_dumb          8286579 ns/iter (+/- 163110)
large_quicksort_insertion     8223799 ns/iter (+/- 188353)
large_quicksort_smart         8307287 ns/iter (+/- 154722)
large_std                     6113112 ns/iter (+/- 23370)
same_bogo                        3883 ns/iter (+/- 6)
same_bubble                      4394 ns/iter (+/- 8)
same_heapsort_down              10125 ns/iter (+/- 14)
same_heapsort_up                11256 ns/iter (+/- 499)
same_insertion                   4923 ns/iter (+/- 8)
same_mergesort                  37884 ns/iter (+/- 575)
same_mergesort_insertion        25502 ns/iter (+/- 45)
same_quicksort_dumb            693800 ns/iter (+/- 1310)
same_quicksort_insertion       695169 ns/iter (+/- 1461)
same_quicksort_smart           695203 ns/iter (+/- 932)
same_selection                 536685 ns/iter (+/- 1853)
same_std                        15378 ns/iter (+/- 45)
sorted_bogo                      3982 ns/iter (+/- 13)
sorted_bubble                    4396 ns/iter (+/- 77)
sorted_heapsort_down            44740 ns/iter (+/- 1003)
sorted_heapsort_up              50721 ns/iter (+/- 150)
sorted_insertion                 4881 ns/iter (+/- 15)
sorted_mergesort                37888 ns/iter (+/- 1429)
sorted_mergesort_insertion      25596 ns/iter (+/- 64)
sorted_quicksort_dumb          561639 ns/iter (+/- 1818)
sorted_quicksort_insertion      25458 ns/iter (+/- 92)
sorted_quicksort_smart          25330 ns/iter (+/- 122)
sorted_selection               536804 ns/iter (+/- 2146)
sorted_std                      15362 ns/iter (+/- 66)
```

