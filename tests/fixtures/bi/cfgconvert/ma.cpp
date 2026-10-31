#define QUOTE(x) #x
#define TWO(a,b) a##b
#define FOO bar
#define NONE() none
class A {
 a = QUOTE(a,b);
 b = TWO(x);
 c = TWO(x,y,z);
 d = FOO(1);
 e = NONE();
 f = QUOTE([1,2]);
};
