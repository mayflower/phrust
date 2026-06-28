--TEST--
Generated reflection.attributes: ReflectionAttribute exposes names, arguments, and repeat metadata
--DESCRIPTION--
module: reflection.attributes
generated timestamp: 20260628T000000Z
generator version: prompt21-reflection-v1
reason: ReflectionAttribute MVP covers class, method, property, and parameter attribute metadata without instantiation.
--FILE--
<?php
#[P21Attr("class", 7)]
class P21AttrTarget {
    #[P21Attr("prop")]
    public string $name;

    #[P21Attr("method")]
    public function run(): void {}
}

function p21_attr_fn(#[P21Attr("param")] int $id): void {}

$classAttrs = (new ReflectionClass(P21AttrTarget::class))->getAttributes();
$methodAttrs = (new ReflectionClass(P21AttrTarget::class))->getMethod("run")->getAttributes();
$propAttrs = (new ReflectionClass(P21AttrTarget::class))->getProperty("name")->getAttributes();
$paramAttrs = (new ReflectionFunction("p21_attr_fn"))->getParameters()[0]->getAttributes();
echo $classAttrs[0]->getName(), ":", $classAttrs[0]->getArguments()[0], ":", $classAttrs[0]->getArguments()[1], ":";
echo $classAttrs[0]->isRepeated() ? "repeat|" : "single|";
echo $methodAttrs[0]->getArguments()[0], "|", $propAttrs[0]->getArguments()[0], "|", $paramAttrs[0]->getArguments()[0];
?>
--EXPECT--
P21Attr:class:7:single|method|prop|param
