<?php
// stdlib-diff: id=STDLIB_OBJECT_FUNCTION_HANDLING area=stdlib expect=pass
class StdlibBox {
    public $pub = 'P';
    protected $prot = 'R';
    private $priv = 'V';

    public function objectVars() {
        $vars = get_object_vars($this);
        return $vars['pub'] . $vars['prot'] . $vars['priv'];
    }

    public function mangledVars() {
        $r = '';
        $v = '';
        foreach (get_mangled_object_vars($this) as $key => $value) {
            if ($value === 'R') { $r = $value; }
            if ($value === 'V') { $v = $value; }
        }
        return $r . $v;
    }

    public static function classVars() {
        $vars = get_class_vars('StdlibBox');
        return $vars['pub'] . $vars['prot'] . $vars['priv'];
    }

    public function visibleMethod() {
        return 'visible';
    }

    protected function protectedMethod() {}
    private function privateMethod() {}
}

function stdlib_join_args($a, $b = 'D') {
    return $a . $b . ':' . func_num_args() . ':' . func_get_arg(0) . ':' . count(func_get_args());
}

function stdlib_named_args($a, $b) {
    return $a . $b;
}

class StdlibCallTarget {
    public static function target($value) {
        return 'S' . $value;
    }

    public static function forward($value) {
        return forward_static_call(['StdlibCallTarget', 'target'], $value);
    }
}

$box = new StdlibBox();
$outside = get_object_vars($box);
$methods = get_class_methods('StdlibBox');
$classVars = get_class_vars('StdlibBox');

echo $outside['pub'], "\n";
echo array_key_exists('prot', $outside) ? "bad\n" : "no-prot\n";
echo $box->objectVars(), "\n";
echo $box->mangledVars(), "\n";
echo StdlibBox::classVars(), "\n";
echo in_array('visibleMethod', $methods, true) ? "method\n" : "missing\n";
echo in_array('protectedMethod', $methods, true) ? "bad\n" : "no-prot-method\n";
echo array_key_exists('priv', $classVars) ? "bad\n" : "no-priv-var\n";
echo call_user_func('stdlib_join_args', 'A', 'B'), "\n";
echo call_user_func_array('stdlib_named_args', ['b' => 'B', 'a' => 'A']), "\n";
echo call_user_func(['StdlibCallTarget', 'target'], 'X'), "\n";
echo StdlibCallTarget::forward('Y'), "\n";
