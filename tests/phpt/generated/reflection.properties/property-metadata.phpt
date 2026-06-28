--TEST--
Generated reflection.properties: ReflectionProperty exposes declaring class, visibility, static, readonly, and type
--DESCRIPTION--
module: reflection.properties
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionProperty MVP covers name, declaring class, visibility, static, readonly, and available type metadata.
--FILE--
<?php
class P21PropertyTarget {
    public readonly int $id;
    protected static string $name;
}

$class = new ReflectionClass(P21PropertyTarget::class);
$id = $class->getProperty("id");
$name = $class->getProperty("name");
echo $id->getName(), ":", $id->getDeclaringClass()->getName(), ":";
echo $id->isPublic() ? "public:" : "notpublic:";
echo $id->isReadOnly() ? "readonly:" : "mutable:";
echo $id->getType()->getName(), "|";
echo $name->getName(), ":";
echo $name->isProtected() ? "protected:" : "notprotected:";
echo $name->isStatic() ? "static:" : "instance:";
echo $name->getType()->getName();
?>
--EXPECT--
id:P21PropertyTarget:public:readonly:int|name:protected:static:string
