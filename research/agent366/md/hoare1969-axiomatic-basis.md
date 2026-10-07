An Axiomatic Basis for
Computer Programming

C. A. R. Hoare
The Queen’s University of Belfast,* Northern Ireland

In this paper an attempt is made to explore the logical founda-
tions of computer programming by use of techniques which
were first applied in the study of geometry and have later
been extended to other branches of mathematics. This in-
volves the elucidation of sets of axioms and rules of inference
which can be used in proofs of the properties of computer
programs. Examples are given of such axioms and rules, and
a formal proof of a simple theorem is displayed. Finally, it is
argued that important advantages, both theoretical and prac-
tical, may follow from a pursuance of these topics.

KEY WORDS AND PHRASES: axiomatic method, theory of programming’
proofs of programs, formal language definition, programming language
design, machine-independent programming, program documentation

CR CATEGORY: 4.0, 4.21, 4.22, 5.20, 5.21, 5.23, 5.24

1. Introduction

Computer programming is an exact science in that all
the properties of a program and all the consequences of
executing it in any given environment can, in principle,
be found out from the text of the program itself by means
of purely deductive reasoning. Deductive reasoning in-
volves the application of valid rules of inference to sets of
valid axioms. It is therefore desirable and interesting to
elucidate the axioms and rules of inference which underlie
our reasoning about computer programs. The exact choice
of axioms will to some extent depend on the choice of
programming language. For illustrative purposes, this
paper is confined to a very simple language, which is effec-
tively a subset of all current procedure-oriented languages.

2. Computer Arithmetic

The first requirement in valid reasoning about a pro-
gram is to know the properties of the elementary operations
which it invokes, for example, addition and multiplication
of integers. Unfortunately, in several respects computer
arithmetic is not the same as the arithmetic familiar to
mathematicians, and it is necessary to exercise some care
in selecting an appropriate set of axioms. For example, the
axioms displayed in Table I are rather a small selection
of axioms relevant to integers. From this incomplete set

* Department of Computer Science

576 Communications of the ACM

of axioms it is possible to deduce such simple theorems as:
x=x+yx0
ysrDr+yXq=r-ytyx d+
The proof of the second of these is:
A5 (r—y)+tyX (+4)
=(r—y)+yXl+yXq)

A9 =(r-y)+ Y+yXQq)
A3 =(r-—y)+y)+yXq
A6 =r-+yXq _ providedy <r

The axioms Al to A9 are, of course, true of the tradi-
tional infinite set of integers in mathematics. However,
they are also true of the finite sets of ‘integers’? which are
manipulated by computers provided that they are con-
fined to nonnegative numbers. Their truth is independent
of the size of the set; furthermore, it is largely independent
of the choice of technique applied in the event of “over-
flow’; for example:

(1) Strict interpretation: the result of an overflowing
operation does not exist; when overflow occurs, the offend-
ing program never completes its operation. Note that in
this case, the equalities of Al to A9 are strict, in the sense
that both sides exist or fail to exist together.

(2) Firm boundary: the result of an overflowing opera-
tion is taken as the maximum value represented.

(3) Modulo arithmetic: the result of an overflowing
operation is computed modulo the size of the set of integers
represented.

These three techniques are illustrated in Table II by
addition and multiplication tables for a trivially small
model in which 0, 1, 2, and 3 are the only integers repre-
sented.

It is interesting to note that the different systems satisfy-
ing axioms Al to A9 may be rigorously distinguished from
each other by choosing a particular one of a set of mutually
exclusive supplementary axioms. For example, infinite
arithmetic satisfies the axiom:

Al0; 7AaVy (y < x),
where all finite arithmetics satisfy:
Al0r Wz (x < max)

where “max” denotes the largest integer represented.
Similarly, the three treatments of overflow may be
distinguished by a choice of one of the following axioms
relating to the value of max + 1:
Alls nae (@ = max+ 1) (strict interpretation )
All;

Ally max+1=0

max + 1 = max (firm boundary )
(modulo arithmetic )

Having selected one of these axioms, it is possible to
use it in deducing the properties of programs; however,

Volume 12 / Number 10 / October, 1969
TABLE I
Al «+ty=yte2 addition is commutative
A2 z«Xy=yXo2 multiplication is commut-
ative
A8 @ty)te=2e24+(y4+2) addition is associative
Ad (@Xy)X2=2%X (yX 2) multiplication is associa-
tive
A5 «£X yt2z)=x2X y+2xXz~ multiplication distrib-

utes through addition

AG y<tDdDe-yt+y=z addition cancels subtrac-

tion

A7 zc£+0=2

A8 «xX0=0

AO «tX1l=2

TABLE II
1. Strict Interpretation
+ 012 8 x 012 8
0 012 0 0000
1 123 * 1 0 12 8
3 30 * x 3 03 * x*
* nonexistent
2. Firm Boundary
+ 012 83 x 01 2 8
0 012 8 0 000 0
1 12 3 8 1 01 2 8
2 23 3 3 2 02 3 8
3 3 3 3 3 3 0 3 3 3
8. Modulo Arithmetic

+ 012 8 012 8
0 012 8 0 0000
1 123 0 1 01 2 8
2 23 01 2 02 0 2
3 301 2 3 0321

these properties will not necessarily obtain, unless the
program is executed on an implementation which satisfies
the chosen axiom.

3. Program Execution

As mentioned above, the purpose of this study is to
provide a logical basis for proofs of the properties of a
program. One of the most important properties of a pro-
gram is whether or not it carries out its intended function.
The intended function of a program, or part of a program,
can be specified by making general assertions about the
values which the relevant variables will take after execution
of the program. These assertions will usually not ascribe
particular values to each variable, but will rather specify
certain general properties of the values and the relation-
ships holding between them. We use the normal notations

Volume 12 / Number 10 / October, 1969

of mathematical logic to express these assertions, and the
familiar rules of operator precedence have been used
wherever possible to improve legibility.

In many cases, the validity of the results of a program
(or part of a program) will depend on the values taken
by the variables before that program is initiated. These
initial preconditions of successful use can be specified by
the same type of general assertion as is used to describe
the results obtained on termination. To state the required
connection between a precondition (P), a program (Q)
and a description of the result of its execution (R), we
introduce a new notation:

PQ} R.

This may be interpreted “If the assertion P is true before
initiation of a program Q, then the assertion R will be
true on its completion.” If there are no preconditions im-
posed, we write true {Q}R."

The treatment given below is essentially due to Floyd
[8] but is applied to texts rather than flowcharts.

3.1. Axiom or ASSIGNMENT

Assignment is undoubtedly the most characteristic fea-
ture of programming a digital computer, and one that
most clearly distinguishes it from other branches of mathe-
matics. It is surprising therefore that the axiom governing
our reasoning about assignment is quite as simple as any
to be found in elementary logic.

Consider the assignment statement:

xi=f
where
xis an identifier for a simple variable;
f is an expression of a programming language without
side effects, but possibly containing z.

Now any assertion P (x) which is to be true of (the value
of) x after the assignment is made must also have been
true of (the value of) the expression f, taken before the
assignment is made, i.e. with the old value of x. Thus
if P (x) is to be true after the assignment, then P(f) must
be true before the assignment. This fact may be expressed
more formally:

DO Axiom of Assignment

Pola := f} P
where

x is a variable identifier;

f is an expression;

Pois obtained from P by substituting f for all occur-

rences of x.

It may be noticed that D0 is not really an axiom at all,
but rather an axiom schema, describing an infinite set of
axioms which share a common pattern. This pattern is
described in purely syntactic terms, and it is easy to
check whether any finite text conforms to the pattern,
thereby qualifying as an axiom, which may validly appear
in any line of a proof.

1 If this can be proved in our formal system, we use the familiar
logical symbol for theoremhood: }P {Q} R

Communications of the ACM 577
3.2. RULES OF CONSEQUENCE

In addition to axioms, a deductive science requires at
least one rule of inference, which permits the deduction of
new theorems from one or more axioms or theorems al-
ready proved. A rule of inference takes the form “If +X
and + Y then |Z”, ie. if assertions of the form X and Y
have been proved as theorems, then Z also is thereby
proved as a theorem. The simplest example of an inference
rule states that if the execution of a program Q en-
sures the truth of the assertion FR, then it also ensures the
truth of every assertion logically implied by R. Also, if
P is known to be a precondition for a program Q to pro-
duce result #, then so is any other assertion which logically
implies P. These rules may be expressed more formally:

D1 Rules of Consequence

If }P{Q}R and |R DS then }P{Q}S
If | P{Q}R and |S DP then |S{Q}R

3.3. RuLE oF CoMPosITION

A program generally consists of a sequence of statements
which are executed one after another. The statements may
be separated by a semicolon or equivalent symbol denoting
procedural composition: (Qi; Q2; --- ; Q,). In order to
avoid the awkwardness of dots, it is possible to deal ini-
tially with only two statements (Q: ; Q2), since longer se-
quences can be reconstructed by nesting, thus (Qi; (Q2;
(+++ (Qn-1; Qn) ++-))). The removal of the brackets of
this nest may be regarded as convention based on the
associativity of the “‘;-operator’’, in the same way as brack-
ets are removed from an arithmetic expression (4 + (& +
(+++ (tra + tr) -++))).

The inference rule associated with composition states
that if the proven result of the first part of a program is
identical with the precondition under which the second part
of the program produces its intended result, then the whole
program will produce the intended result, provided that the
precondition of the first part is satisfied.

In more formal terms:

D2 Rule of Composition
If }P{Q:}Ri and }Ri{Q.}R then }P{(Qi; Q)}R

3.4. RuLe or ITERATION
The essential feature of a stored program computer is
the ability to execute some portion of program (S) re-
peatedly until a condition (B) goes false. A simple way of
expressing such an iteration is to adapt the Ancou 60
while notation:
while B do S

In executing this statement, a computer first tests the con-
dition B. If this is false, S is omitted, and execution of the
loop is complete. Otherwise, S is executed and B is tested
again. This action is repeated until B is found to be false.
The reasoning which leads to a formulation of an inference
rule for iteration is as follows. Suppose P to be an assertion
which is always true on completion of S, provided that it is
also true on initiation. Then obviously P will still be true
after any number of iterations of the statement S (even

578 Communications of the ACM

no iterations). Furthermore, it is known that the con-
trolling condition B is false when the iteration finally
terminates. A slightly more powerful formulation is pos-
sible in light of the fact that B may be assumed to be true
on initiation of S:

D3 Rule of Iteration
If }|P A B{S}P then |P{while B do S}nB/AP_

3.5. EXAMPLE

The axioms quoted above are sufficient to construct the
proof of properties of simple programs, for example, a
routine intended to find the quotient g and remainder r
obtained on dividing x by y. All variables are assumed to
range over a set of nonnegative integers conforming to the
axioms listed in Table I. For simplicity we use the trivial
but inefficient method of successive subtraction. The pro-
posed program is:

((7:= 2; q:=0); while

y<rdo(r:=r—y; q:=1+4q))

An important property of this program is that when it
terminates, we can recover the numerator x by adding to
the remainder r the product of the divisor y and the quo-
tient gq (ie. 2 = r+ y X q). Furthermore, the remainder
is less than the divisor. These properties may be expressed
formally:

true {Q} ny <rAvart+yXq

where Q stands for the program displayed above. This
expresses a necessary (but not sufficient) condition for
the “correctness” of the program.

A formal proof of this theorem is given in Table III.
Like all formal proofs, it is excessively tedious, and it
would be fairly easy to introduce notational conventions
which would significantly shorten it. An even more power-
ful method of reducing the tedium of formal proofs is to
derive general rules for proof construction out of the simple
rules accepted as postulates. These general rules would be
shown to be valid by demonstrating how every theorem
proved with their assistance could equally well (if more
tediously ) have been proved without. Once a powerful set
of supplementary rules has been developed, a ‘formal
proof” reduces to little more than an informal indication
of how a formal proof could be constructed.

4. General Reservations

The axioms and rules of inference quoted in this paper
have implicitly assumed the absence of side effects of the
evaluation of expressions and conditions. In proving prop-
erties of programs expressed in a language permitting side
effects, it would be necessary to prove their absence in
each case before applying the appropriate proof technique.
If the main purpose of a high level programming language
is to assist in the construction and verification of correct
programs, it is doubtful whether the use of functional
notation to call procedures with side effects is a genuine
advantage.

Another deficiency in the axioms and rules quoted above

Volume 12 / Number 10 / October, 1969
is that they give no basis for a proof that a program suc-
cessfully terminates. Failure to terminate may be due to an
infinite loop; or it may be due to violation of an imple-
mentation-defined limit, for example, the range of numeric
operands, the size of storage, or an operating system time
limit. Thus the notation “P{Q}R” should be interpreted
“provided that the program successfully terminates, the
properties of its results are described by R.” It is fairly
easy to adapt the axioms so that they cannot be used to
predict the “results” of nonterminating programs; but the
actual use of the axioms would now depend on knowledge
of many implementation-dependent features, for example,
the size and speed of the computer, the range of numbers,
and the choice of overflow technique. Apart from proofs of
the avoidance of infinite loops, it is probably better to
prove the “conditional” correctness of a program and rely
on an implementation to give a warning if it has had to

TABLE III
Line
number Formal proof Justification
1 trueD,z=2z2+yxX0 Lemma 1
2 z=a2+yXO{r:=azc}e=rt+tyx0 DO
3 zx=r+yX0{¢q:=Or=r+yxq DO
4 true {r:= az} 2=r+yxX0 D1 (1, 2)
5 true {r:= 2; g:=OJrx=r+yxXq D2 (4, 3)
6 zZ@=r+yXqAy<rdzr=
(r-y) +y X (+9) Lemma 2
7 2 = (r-y) +yX (t+g){r i= r—-yja =
r+yX (+9) DO
8 w=rt+yX I+gi¢ := 1+g}e =
r+yXq DO
9 x= (r-y) +yX I+a){r i= r—y;
gq:=ltgyr=rt+yXq D2 (7, 8)
10 zx=r+yXqANyK<r{r:=r—-y;
q:=ltq)e=r+yxXq D1 6, 9)
11 z=r+yX q {while y<r do
(r= r-y; g:= 14+}
ay<rAt=r+yXxXq D3 (10)
12 true {((r := 2; q:= 0); while y < rdo
(i= r-y; g:=14+q)} ny¥<rAgs
r+yXq D2 (5, 11)

Nores

1. The left hand column is used to number the lines, and the
right hand column to justify each line, by appealing to an axiom,
a lemma or a rule of inference applied to one or two previous
lines, indicated in brackets. Neither of these columns is part
of the formal proof. For example, line 2 is an instance of the
axiom of assignment (D0); line 12 is obtained from lines 5 and 11
by application of the rule of composition (D2).

2. Lemma 1 may be proved from axioms A7 and A8.

3. Lemma 2 follows directly from the theorem proved in Sec. 2.

Volume 12 / Number 10 / October, 1969

abandon execution of the program as a result of violation
of an implementation limit.

Finally it is necessary to list some of the areas which have
not been covered: for example, real arithmetic, bit and
character manipulation, complex arithmetic, fractional
arithmetic, arrays, records, overlay definition, files, input/
output, declarations, subroutines, parameters, recursion,
and parallel execution. Even the characterization of integer
arithmetic is far from complete. There does not appear to
be any great difficulty in dealing with these points, pro-
vided that the programming language is kept simple.
Areas which do present real difficulty are labels and jumps,
pointers, and name parameters. Proofs of programs which
made use of these features are likely to be elaborate, and
it is not surprising that this should be reflected in the
complexity of the underlying axioms.

5. Proofs of Program Correctness

The most important property of a program is whether it
accomplishes the intentions of its user. If these intentions
can be described rigorously by making assertions about the
values of variables at the end (or at intermediate points ) of
the execution of the program, then the techniques described
in this paper may be used to prove the correctness of the
program, provided that the implementation of the pro-
gramming language conforms to the axioms and rules which
have been used in the proof. This fact itself might also be
established by deductive reasoning, using an axiom set
which describes the logical properties of the hardware
circuits. When the correctness of a program, its compiler,
and the hardware of the computer have all been established
with mathematical certainty, it will be possible to place
great reliance on the results of the program, and predict
their properties with a confidence limited only by the
reliability of the electronics.

The practice of supplying proofs for nontrivial programs
will not become widespread until considerably more power-
ful proof techniques become available, and even then will
not be easy. But the practical advantages of program prov-
ing will eventually outweigh the difficulties, in view of the
increasing costs of programming error. At present, the
method which a programmer uses to convince himself of
the correctness of his program is to try it out in particular
cases and to modify it if the results produced do not cor-
respond to his intentions. After he has found a reasonably
wide variety of example cases on which the program seems
to work, he believes that it will always work. The time
spent in this program testing is often more than half the
time spent on the entire programming project; and with a
realistic costing of machine time, two thirds (or more) of
the cost of the project is involved in removing errors during
this phase.

The cost of removing errors discovered after a program
has gone into use is often greater, particularly in the case
of items of computer manufacturer’s software for which a
large part of the expense is borne by the user. And finally,
the cost of error in certain types of program may be almost

Communications of the ACM 579
incalculable—a lost spacecraft, a collapsed building, a
crashed aeroplane, or a world war. Thus the practice of
program proving is not only a theoretical pursuit, followed
in the interests of academic respectability, but a serious
recommendation for the reduction of the costs associated
with programming error.

The practice of proving programs is likely to alleviate
some of the other problems which afflict the computing
world. For example, there is the problem of program docu-
mentation, which is essential, firstly, to inform a potential
user of a subroutine how to use it and what it accomplishes,
and secondly, to assist in further development when it
becomes necessary to update a program to meet changing
circumstances or to improve it in the light of increased
knowledge. The most rigorous method of formulating the
purpose of a subroutine, as well as the conditions of its
proper use, is to make assertions about the values of vari-
ables before and after its execution. The proof of the cor-
rectness of these assertions can then be used as a lemmain
the proof of any program which calls the subroutine. Thus,
in a large program, the structure of the whole can be clearly
mirrored in the structure of its proof. Furthermore, when
it becomes necessary to modify a program, it will always be
valid to replace any subroutine by another which satisfies
the same criterion of correctness. Finally, when examining
the detail of the algorithm, it seems probable that the proof
will be helpful in explaining not only what is happening
but why.

Another problem which can be solved, insofar as it is
soluble, by the practice of program proofs is that of trans-
ferring programs from one design of computer to another.
Even when written in a so-called machine-independent
programming language, many large programs inadvert-
ently take advantage of some machine-dependent prop-
erty of a particular implementation, and unpleasant and
expensive surprises can result when attempting to transfer
it to another machine. However, presence of a machine-
dependent feature will always be revealed in advance by
the failure of an attempt to prove the program from ma-
chine-independent axioms. The programmer will then have
the choice of formulating his algorithm in a machine-
independent fashion, possibly with the help of environment
enquiries; or if this involves too much effort or inefficiency,
he can deliberately construct a machine-dependent pro-
gram, and rely for his proof on some machine-dependent
axiom, for example, one of the versions of All (Section 2).
In the latter case, the axiom must be explicitly quoted as
one of the preconditions of successful use of the program.
The program can still, with complete confidence, be trans-
ferred to any other machine which happens to satisfy the
same machine-dependent axiom; but if it becomes neces-
sary to transfer it to an implementation which does not,
then all the places where changes are required{will be
clearly annotated by the fact that the proof at that point
appeals to the truth of the offending machine-dependent
axiom.

Thus the practice of proving programs would seem to

580 Communications of the ACM

lead to solution of three of the most pressing problems in
software and programming, namely, reliability, documen-
tation, and compatibility. However, program proving, cer-
tainly at present, will be difficult even for programmers of
high caliber; and may be applicable only to quite simple
program designs. As in other areas, reliability can be pur-
chased only at the price of simplicity.

6. Formal Language Definition

A high level programming language, such as ALGOL,
Fortran, or Coso1, is usually intended to be implemented
on a variety of computers of differing size, configuration,
and design. It has been found a serious problem to define
these languages with sufficient rigour to ensure compat-
ibility among all implementors. Since the purpose of com-
patibility is to facilitate interchange of programs ex-
pressed in the language, one way to achieve this would be to
insist that all implementations of the language shall “‘sat-
isfy”? the axioms and rules of inference which underlie
proofs of the properties of programs expressed in the
language, so that all predictions based on these proofs will
be fulfilled, except in the event of hardware failure. In
effect, this is equivalent to accepting the axioms and rules
of inference as the ultimately definitive specification of the
meaning of the language.

Apart from giving an immediate and possibly even
provable criterion for the correctness of an implementation,
the axiomatic technique for the definition of programming
language semantics appears to be like the formal syntax of
the Atco 60 report, in that it is sufficiently simple to be
understood both by the implementor and by the reasonably
sophisticated user of the language. It is only by bridging
this widening communication gap in a single document
(perhaps even provably consistent) that the maximum
advantage can be obtained from a formal language def-
inition.

Another of the great advantages of using an axiomatic
approach is that axioms offer a simple and flexible tech-
nique for leaving certain aspects of a language undefined,
for example, range of integers, accuracy of floating point,
and choice of overflow technique. This is absolutely es-
sential for standardization purposes, since otherwise the
language will be impossible to implement efficiently on
differing hardware designs. Thus a programming language
standard should consist of a set of axioms of universal
applicability, together with a choice from a set of supple-
mentary axioms describing the range of choices facing an
implementor. An example of the use of axioms for this
purpose was given in Section 2.

Another of the objectives of formal language definition
is to assist in the design of better programming languages.
The regularity, clarity, and ease of implementation of the
Atcot 60 syntax may at least in part be due to the use of
an elegant formal technique for its definition. The use of
axioms may lead to similar advantages in the area of
“semantics,” since it seems likely that a language which can

(Continued on p. 583)

Volume 12 / Number 10 / October, 1969
C. A. R. HOARE—coni’d from page 580

be described by a few “‘self-evident” axioms from which
proofs will be relatively easy to construct will be preferable
to a language with many obscure axioms which are dif-
ficult to apply in proofs. Furthermore, axioms enable the
language designer to express his general intentions quite
simply and directly, without the mass of detail which
usually accompanies algorithmic descriptions. Finally, ax-
ioms can be formulated in a manner largely independent
of each other, so that the designer can work freely on one
axiom or group of axioms without fear of unexpected in-
teraction effects with other parts of the language.

Acknowledgments. Many axiomatic treatments of com-
puter programming [1, 2, 3] tackle the problem of proving
the equivalence, rather than the correctness, of algorithms.
Other approaches [4, 5] take recursive functions rather
than programs as a starting point for the theory. The
suggestion to use axioms for defining the primitive opera-
tions of a computer appears in [6, 7]. The importance of
program proofs is clearly emphasized in [9], and an in-
formal technique for providing them is described. The
suggestion that the specification of proof techniques pro-
vides an adequate formal definition of a programming
language first appears in [8]. The formal treatment of pro-
gram execution presented in this paper is clearly derived
from Floyd. The main contributions of the author appear
tobe: (1) a suggestion that axioms may provide a simple
solution to the problem of leaving certain aspects of a

Volume 12 / Number 10 / October, 1969

language undefined; (2) a comprehensive evaluation of
the possible benefits to be gained by adopting this approach
both for program proving and for formal language defini-
tion.

However, the formal material presented here has only
an expository status and represents only a minute propor-
tion of what remains to be done. It-is hoped that many of
the fascinating problems involved will be taken up by
others.

Recetvep NoveMBER, 1968; REviseD May, 1969

REFERENCES

. Yanov, Yu I. Logical operator schemes. Kybernetika 1, (1958).

2. Iearasui, S. An axiomatic approach to equivalence problems
of algorithms with applications. Ph.D. Thesis 1964. Rep.
Compt. Centre, U. Tokyo, 1968, pp. 1-101.

3. pm Bakker, J. W. Axiomatics of simple assignment statements.
M.R. 94, Mathematisch Centrum, Amsterdam, June 1968.

4. McCartuy, J. Towards a mathematical theory of computation.
Proc. IFIP Cong. 1962, North Holland Pub. Co., Amsterdam,
1963.

5. Burstatu, R. Proving properties of programs by structural in-
duction. Experimental Programming Reports: No. 17 DMIP,
Edinburgh, Feb. 1968.

6. vAN WIJNGAARDEN, A. Numerical analysis as an independent
science. BIT 6 (1966), 66-81.

7. Lasxt, J. Sets and other types. ALGOL Bull. 27, 1968.

8. Froyp, R. W. Assigning meanings to programs. Proc. Amer.
Math. Soc. Symposia in Applied Mathematics, Vol. 19, pp.
19-31.

9. Naur, P. Proof of algorithms by general snapshots. BIT 6

(1966), 310-316.

_

Communications of the ACM 583
