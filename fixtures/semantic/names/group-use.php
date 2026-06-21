<?php
namespace App;

use Vendor\Package\{ClassA, ClassB as B};
use Vendor\Package\{function helper_two, const OTHER_VALUE};

echo ClassA::class;
echo B::class;
helper_two();
echo OTHER_VALUE;
