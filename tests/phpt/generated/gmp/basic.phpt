--TEST--
gmp BigInt facade basics
--SKIPIF--
<?php if (!extension_loaded("gmp")) die("skip gmp extension not loaded"); ?>
--FILE--
<?php
echo gmp_strval(gmp_add(gmp_init("0xff", 0), "1")), "\n";
echo gmp_strval(gmp_init("0b1010", 0)), "\n";
echo gmp_strval(gmp_init("010", 0)), "\n";
echo gmp_strval(gmp_mul("12", "12")), "\n";
echo gmp_strval(gmp_div_q("7", "2")), "\n";
$qr = gmp_div_qr("7", "2");
echo gmp_strval($qr[0]), ":", gmp_strval($qr[1]), "\n";
echo gmp_strval(gmp_div_r("7", "2")), "\n";
echo gmp_strval(gmp_mod("-3", "5")), "\n";
echo gmp_cmp("10", "9"), "\n";
echo gmp_strval(gmp_pow("2", 8)), "\n";
echo gmp_strval(gmp_powm("4", "13", "497")), "\n";
echo gmp_strval(gmp_gcd("84", "30")), "\n";
echo gmp_strval(gmp_lcm("84", "30")), "\n";
echo gmp_strval(gmp_invert("3", "11")), "\n";
$ext = gmp_gcdext("30", "21");
echo gmp_strval($ext["g"]), ":", gmp_strval($ext["s"] * 30 + $ext["t"] * 21), "\n";
$sqrt = gmp_sqrtrem("20");
echo gmp_strval($sqrt[0]), ":", gmp_strval($sqrt[1]), "\n";
$root = gmp_rootrem("28", 3);
echo gmp_strval($root[0]), ":", gmp_strval($root[1]), "\n";
echo gmp_perfect_square("144") ? "square\n" : "not-square\n";
echo gmp_perfect_power("27") ? "power\n" : "not-power\n";
echo gmp_prob_prime("13"), ":", gmp_strval(gmp_nextprime("14")), "\n";
echo gmp_strval(gmp_and("10", "12")), ":", gmp_strval(gmp_or("10", "12")), ":", gmp_strval(gmp_xor("10", "12")), ":", gmp_strval(gmp_com("10")), "\n";
echo (gmp_testbit("10", 1) ? "bit\n" : "no-bit\n");
echo gmp_popcount("10"), ":", gmp_hamdist("10", "12"), ":", gmp_scan1("8", 0), ":", gmp_scan0("7", 0), "\n";
echo gmp_strval(gmp_fact("6")), ":", gmp_strval(gmp_binomial("6", 2)), "\n";
echo gmp_strval(gmp_import("\x01\x00")), ":", bin2hex(gmp_export("258")), "\n";
?>
--EXPECT--
256
10
8
144
3
3:1
1
2
1
256
445
6
420
4
3:3
4:4
3:1
square
power
2:17
8:14:6:-11
bit
2:2:3:3
720:15
256:0102
