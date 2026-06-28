--TEST--
Generated reflection.classes: ReflectionClass exposes names, parent, interfaces, and members
--DESCRIPTION--
module: reflection.classes
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionClass MVP covers class names, namespace, flags, parent, interfaces, methods, properties, and constants.
--FILE--
<?php
namespace P21\Ns;

class Base {}
interface I {}
abstract class Child extends Base implements I {
    public const C = 1;
    public string $name;
    public function run(): void {}
}

$class = new \ReflectionClass(Child::class);
echo $class->getName(), "|", $class->getShortName(), "|", $class->getNamespaceName(), "|";
echo $class->inNamespace() ? "namespace|" : "global|";
echo $class->isAbstract() ? "abstract|" : "concrete|";
echo $class->isInterface() ? "iface|" : "class|";
echo $class->isEnum() ? "enum|" : "notenum|";
echo $class->getParentClass()->getName(), "|", $class->getInterfaceNames()[0], "|";
echo count($class->getMethods()), ":", count($class->getProperties()), ":", count($class->getConstants());
?>
--EXPECT--
P21\Ns\Child|Child|P21\Ns|namespace|abstract|class|notenum|P21\Ns\Base|P21\Ns\I|1:1:1
