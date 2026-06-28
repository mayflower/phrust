--TEST--
Generated reflection.methods: ReflectionMethod exposes declaring class and modifiers
--DESCRIPTION--
module: reflection.methods
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionMethod MVP covers name, declaring class, visibility, static, final, and parameter metadata.
--FILE--
<?php
class P21MethodTarget {
    final public function run(int $x): int { return $x; }
    protected static function make(): void {}
}

$class = new ReflectionClass(P21MethodTarget::class);
$run = $class->getMethod("run");
$make = $class->getMethod("make");
echo $run->getName(), ":", $run->getDeclaringClass()->getName(), ":";
echo $run->isPublic() ? "public:" : "notpublic:";
echo $run->isFinal() ? "final:" : "notfinal:";
echo $run->isStatic() ? "static:" : "instance:";
printf("%08x:", $run->getModifiers());
echo $run->getNumberOfParameters(), ":", $run->getReturnType()->getName(), "|";
echo $make->getName(), ":";
echo $make->isProtected() ? "protected:" : "notprotected:";
echo $make->isStatic() ? "static" : "instance";
printf(":%08x", $make->getModifiers());
?>
--EXPECT--
run:P21MethodTarget:public:final:instance:00000021:1:int|make:protected:static:00000012
