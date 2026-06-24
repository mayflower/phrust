--TEST--
Phase 9 generated regression: object numeric casts warn and return one
--DESCRIPTION--
original php-src path: Zend/tests/type_coercion/type_casts/cast_to_int.phpt
original source hash: 5a2a4047d1ed8f4d7c9e5c6c9e3e0960517dd8330be2098bed3a3193aa1fd755
related original: Zend/tests/type_coercion/type_casts/cast_to_double.phpt
related original source hash: e4bb7e0ea2813e7cbf40dd44950869e67c87a8562d04a5be723b9bc67f9a2f70
generated timestamp: 20260624T000000Z
generator version: phase9-operators-conversions-v1
reason: reduced object-to-int/object-to-float cast regression generated from reference output
--FILE--
<?php
class test {
    function __toString() {
        return "10";
    }
}

$o = new test;
var_dump((int) $o);
var_dump((float) $o);
--EXPECTF--

Warning: Object of class test could not be converted to int in %s on line %d
int(1)

Warning: Object of class test could not be converted to float in %s on line %d
float(1)
