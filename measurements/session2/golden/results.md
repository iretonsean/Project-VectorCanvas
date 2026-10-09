| state | path | mean | p99.9 | max | px not identical | max ch diff | verdict |
|---|---|---|---|---|---|---|---|
| L01_idle | gl-llvmpipe | 0.000126 | 0.01866 | 0.0324 | 3069 | 2 | PASS |
| L01_idle | web-webgl2 | 0.000110 | 0.01782 | 0.0301 | 2440 | 1 | PASS |
| F01_empty | gl-llvmpipe | 0.000365 | 0.05292 | 0.0559 | 7129 | 1 | FAIL |
| F01_empty | web-webgl2 | 0.000363 | 0.05329 | 0.0555 | 7044 | 1 | FAIL |
| F01_error | gl-llvmpipe | 0.000421 | 0.05326 | 0.0555 | 8512 | 1 | FAIL |
| F01_error | web-webgl2 | 0.000408 | 0.05326 | 0.0555 | 8425 | 1 | FAIL |
| F01_zh_preedit | gl-llvmpipe | 0.000335 | 0.05255 | 0.0559 | 6791 | 1 | FAIL |
| F01_zh_preedit | web-webgl2 | 0.000319 | 0.05275 | 0.0555 | 6722 | 1 | FAIL |
