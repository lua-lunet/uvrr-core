# Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

Luca Geatti ✉ 🚿 🟢

University of Udine, Italy

Fondazione Bruno Kessler, Trento, Italy

Nicola Gigante ✉ 🚿 🟢

Free University of Bozen-Bolzano, Italy

Angelo Montanari ✉ 🚿 🟢

University of Udine, Italy

Gabriele Venturato ✉ 🚿 🟢

University of Udine, Italy

KU Leuven, Belgium

## Abstract

LTL+Past is the extension of Linear Temporal Logic (LTL) supporting past temporal operators. The addition of the past does not add expressive power, but does increase the usability of the language both in formal verification and in artificial intelligence, e.g., in the context of multi-agent systems. In this paper, we add the support of past operators to BLACK, a satisfiability checker for LTL based on a SAT encoding of a tree-shaped tableau system. We implement two ways of supporting the past in the tool. The first one is an equisatisfiable translation that removes the past operators, obtaining a future-only formula that can be solved with the original LTL engine. The second one extends the SAT encoding of the underlying tableau to directly support the tableau rules that deal with past operators. We describe both approaches and experimentally compare the two between themselves and with the nuXmv model checker, obtaining promising results.

2012 ACM Subject Classification Theory of computation → Modal and temporal logics

Keywords and phrases SAT, LTL, LTL+Past, Tableaux

Digital Object Identifier 10.4230/LIPIcs.TIME.2021.8

Acknowledgements The open access publication of this article was supported by the Alpen-Adria-Universität Klagenfurt, Austria. Nicola Gigante acknowledges the partial support of the TOTA project “Temporal Ontologies and Tableaux Algorithms”. Gabriele Venturato acknowledges the partial support of the KU Leuven Research Fund (C14/18/062).

## 1 Introduction

Linear Temporal Logic (LTL) [20] is the de-facto standard temporal specification language in many areas including formal verification [8] and artificial intelligence [11]. Satisfiability checking, that is, deciding whether a given formula admits a model, is a particularly important problem because of its wide range of applications, and one of the first that have been studied [23, 26]. Many techniques and tools have been developed to solve it, ranging from tableau systems [1, 17, 22, 26] to reduction to model checking [5], from temporal resolution [9, 10, 14] to automata-theoretic techniques [16].

The Bounded LTL sAtisfiability Checker, BLACK for short, is a recently developed tool [12] that solves the satisfiability checking problem for LTL by providing a SAT encoding of the one-pass and tree-shaped tableau method for LTL proposed by Reynolds [22]. In an iterative procedure, the tree-shaped tableau is symbolically explored in a breadth-first way through a SAT encoding of its branches of depth at most k, for increasing values of k. The tool proved to be competitive with other state-of-the-art approaches [12].

© Luca Geatti, Nicola Gigante, Angelo Montanari, and Gabriele Venturato;

licensed under Creative Commons License CC-BY 4.0

28th International Symposium on Temporal Representation and Reasoning (TIME 2021).

Editors: Carlo Combi, Johann Eder, and Mark Reynolds; Article No. 8; pp. 8:1–8:17

Leibniz International Proceedings in Informatics

LIPICS Schloss Dagstuhl – Leibniz-Zentrum für Informatik, Dagstuhl Publishing, Germany

---

8:2

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

In this paper, we extend BLACK to support LTL+Past, which enriches LTL with past operators, i.e. temporal modalities that talk about the past of the current time point. Although past operators do not add expressive power to the language, interestingly LTL+Past is exponentially more succinct than LTL, and it is thus able to express some useful properties in a more compact and natural way  \( [19] \) . LTL+Past has been investigated from both theoretical and algorithmic viewpoints  \( [13,17,18,21] \) , in the areas of formal verification  \( [7] \)  and artificial intelligence, e.g., in the context of multi-agent systems (see  \( [3] \)  and references therein).

We present and compare two different methods to support LTL+Past in BLACK.

The first one is a Tseitin-style  \( [25] \)  translation procedure that, given an LTL+Past formula, generates an equisatisfiable LTL formula that can be solved by the original LTL engine of BLACK, or, in principle, by any other tool for LTL satisfiability checking. Since the resulting formula is equisatisfiable, and not equivalent, to the original one, it can avoid the exponential blowup (it causes only a linear size increase). The core idea behind the translation is folklore, but, to the best of our knowledge, this is the first time it is explicitly worked out for LTL, implemented in a tool, and experimentally compared with other approaches.

The second method extends the SAT encoding  \( [12] \)  of the tableau system for LTL described in  \( [13,22] \)  with the ability of directly handling past temporal operators. The resulting encoding successfully supports the claim given in  \( [12,13] \)  that the one-pass and tree-shaped tableau system for LTL, together with its SAT encoding, can be easily extended to other temporal logics. Last but not least, our encoding for the support of past operators is much simpler than similar methods, like, for instance, the one based on virtual unrollings by Latvala et al.  \( [15] \) .

We make a comparison of the two methods, showing that the direct encoding outperforms the translation, although both methods show comparable performance. This makes the direct encoding the preferred way of handling the past in BLACK, but shows that the translation can be a useful and effective preprocessing step to deal with past operators in tools that do not support them natively. Since there is not a standardized set of benchmarks involving past operators in the literature, we introduce some novel sets of formulas for the sake of this comparison.

Finally, we compare both solutions with the nuXmv model checker, which is the only widely available tool, as far as we know, that directly supports past operators. The results are promising, showing BLACK to be competitive.

The paper is organized as follows. Section 2 introduces LTL+Past and Reynolds' tableau [22]. Then, Section 3 and Section 4 describe the translation and the direct encoding, respectively. Finally, Section 5 describes the results of the experimental comparison, and Section 6 concludes the paper.

## 2 Preliminaries

Let \(\Sigma\) be an alphabet of proposition letters. The syntax of an LTL+Past formula \(\phi\) over \(\Sigma\) can be defined as follows:

\[
\phi := p \mid \neg \phi \mid \phi_ {1} \vee \phi_ {2} \mid \phi_ {1} \wedge \phi_ {2} \mid
\]

\[
\mathsf {X} \phi \mid \phi_ {1} \mathcal {U} \phi_ {2} \mid \phi_ {1} \mathcal {R} \phi_ {2} \mid
\]

\[
\mathsf {Y} \phi \mid \mathsf {Z} \phi \mid \phi_ {1} \mathcal {S} \phi_ {2} \mid \phi_ {1} \mathcal {T} \phi_ {2}
\]

Boolean connectives

future temporal operators

past temporal operators

where  \( p \in \Sigma \)  and  \( \phi \) ,  \( \phi_{1} \) , and  \( \phi_{2} \)  are LTL+Past formulas. LTL is the fragment that only uses Boolean connectives and future operators. We denote by LTL[ \( \Sigma \) ] and LTL+Past[ \( \Sigma \) ], respectively, the sets of LTL and LTL+Past formulas built over the alphabet  \( \Sigma \) . Standard shorthands and derived operators are also available, such as  \( \top \equiv p \vee \neg p \) , for some  \( p \in \Sigma \) ,  \( \bot \equiv \neg \top \) ,  \( F\phi \equiv \top U\phi \) ,  \( G\phi \equiv \neg F\neg\phi \) ,  \( O\phi \equiv \top S\phi \) ,  \( H\phi \equiv \neg O\neg\phi \) .

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:3

LTL+Past is interpreted over infinite state sequences \(\overline{\sigma} \in (2^{\Sigma})^{\omega}\). Given a state sequence \(\overline{\sigma} \in (2^{\Sigma})^{\omega}\), the satisfaction of a formula \(\phi\) by \(\overline{\sigma}\) at a time point \(i \geq 0\), denoted as \(\overline{\sigma}, i \models \phi\), is defined as follows:

1. \(\overline{\sigma}, i \models p\) iff \(p \in \sigma_i\);
2. \(\overline{\sigma}, i \models \neg \phi\) iff \(\overline{\sigma}, i \not\models \phi\);
3. \(\overline{\sigma}, i \models \phi_1 \lor \phi_2\) iff \(\overline{\sigma}, i \models \phi_1\) or \(\overline{\sigma}, i \models \phi_2\);
4. \(\overline{\sigma}, i \models \phi_1 \land \phi_2\) iff \(\overline{\sigma}, i \models \phi_1\) and \(\overline{\sigma}, i \models \phi_2\);
5. \(\overline{\sigma}, i \models X\phi\) iff \(\overline{\sigma}, i + 1 \models \phi\);
6. \(\overline{\sigma}, i \models \Upsilon \phi\) iff \(i > 0\) and \(\overline{\sigma}, i - 1 \models \phi\);
7. \(\overline{\sigma}, i \models Z\phi\) iff either \(i = 0\) or \(\overline{\sigma}, i - 1 \models \phi\);
8. \(\overline{\sigma}, i \models \phi_1 \mathcal{U} \phi_2\) iff there exists \(j \geq i\) such that \(\overline{\sigma}, j \models \phi_2\), and \(\overline{\sigma}, k \models \phi_1\) for all \(k\), with \(i \leq k < j\);
9. \(\overline{\sigma}, i \models \phi_1 S \phi_2\) iff there exists \(j \leq i\) such that \(\overline{\sigma}, j \models \phi_2\), and \(\overline{\sigma}, k \models \phi_1\) for all \(k\), with \(j < k \leq i\);
10. \(\overline{\sigma}, i \models \phi_1 \mathcal{R} \phi_2\) iff either \(\overline{\sigma}, j \models \phi_2\) for all \(j \geq i\), or there exists \(k \geq i\) such that \(\overline{\sigma}, k \models \phi_1\) and \(\overline{\sigma}, j \models \phi_2\) for all \(i \leq j \leq k\);
11. \(\overline{\sigma}, i \models \phi_1 \mathcal{T} \phi_2\) iff either \(\overline{\sigma}, j \models \phi_2\) for all \(0 \leq j \leq i\), or there exists \(k \leq i\) such that \(\overline{\sigma}, k \models \phi_1\) and \(\overline{\sigma}, j \models \phi_2\) for all \(i \geq j \geq k\)

We say that a state sequence \(\overline{\sigma}\) satisfies \(\phi\), written \(\overline{\sigma} \models \phi\), if \(\overline{\sigma}, 0 \models \phi\). Observe that the \(\wedge\) connective, the release operator \((\phi_1 \mathcal{R} \phi_2)\), the triggered operator \((\phi_1 \mathcal{T} \phi_2)\), and the weak yesterday operator \((Z\phi)\) can be defined in terms of the \(\vee\) connective, the until operator \((\phi_1 \mathcal{U} \phi_2)\), the since operator \((\phi_1 S \phi_2)\), and the yesterday operator \((Y\phi)\), respectively, but here we consider them as primitive operators since this allows us to put any formula into negation normal form (NNF), which will be useful later. Moreover, note that state sequences have a definite starting point, hence the past is bounded, and we need to distinguish between the yesterday operator \((Y\phi, \phi\) holds at the previous state) and the weak yesterday operator \((Z\phi, \phi\) holds at the previous state, if it exists) as opposed to a single tomorrow operator \((X\phi, \phi\) holds at the next state).

The notion of closure of a formula will be useful later.

▶ Definition 1 (Closure of an LTL+Past formula). Let  \( \psi \)  be an LTL+Past formula built over  \( \Sigma \) . The closure of  \( \psi \)  is the smallest set of formulas  \( \mathcal{C}(\psi) \)  satisfying the following properties:

1. \(\psi \in \mathcal{C}(\psi)\);
2. for each sub-formula \(\psi'\) of \(\psi\), \(\psi' \in \mathcal{C}(\psi)\);
3. for each \(p \in \Sigma\), \(p \in \mathcal{C}(\psi)\) if and only if \(\neg p \in \mathcal{C}(\psi)\);
4. if \(\psi_1\mathcal{U}\phi_2\in \mathcal{C}(\psi)\), then \(\mathsf{X}(\psi_1\mathcal{U}\psi_2)\in \mathcal{C}(\psi)\);
5. if \(\phi_1\mathcal{R}\psi_2\in \mathcal{C}(\psi)\), then \(\mathsf{X}(\psi_1\mathcal{R}\phi_2)\in \mathcal{C}(\psi)\);
6. if \(\psi_1\mathcal{S}\psi_2\in \mathcal{C}(\psi)\), then \(\Upsilon (\psi_1\mathcal{S}\psi_2)\in \mathcal{C}(\psi)\);
7. if \(\psi_1\mathcal{T}\psi_2\in \mathcal{C}(\psi)\), then \(Z(\psi_1\mathcal{T}\psi_2)\in \mathcal{C}(\psi)\).

It is worth pointing out that item 3 of Definition 1 only applies to proposition letters because formulas are assumed to be in NNF.

### The one-pass and tree-shaped tableau for LTL+Past

Let us now briefly describe the tableau system for LTL+Past introduced by Geatti et al. [13], which will be used as the basis for the direct encoding discussed in Section 4. It extends the tableau system for LTL by Reynolds [22]. The latter has the distinctive features of being tree-shaped, as opposed to standard graph-shaped LTL tableaux, e.g., [17], and one-pass, since a single pass is sufficient to either accept or reject a given branch.

TIME 2021

---

8:4

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

Table 1 Tableau expansion rules. When a formula \(\phi\) of one of the types shown in the table is found in the label \(\Gamma\) of a node \(u\), one or two children \(u'\) and \(u''\) are created with the same label as \(u\), but replacing \(\phi\) by the formulas from \(\Gamma_1(\phi)\) and \(\Gamma_2(\phi)\), respectively.

|  Rule | \( \phi \in \Gamma \) | \( \Gamma_1(\phi) \) | \( \Gamma_2(\phi) \)  |
| --- | --- | --- | --- |
|  DISJUNCTION | \( \alpha \lor \beta \) | \( \{ \alpha \} \) | \( \{ \beta \} \)  |
|  CONJUNCTION | \( \alpha \land \beta \) | \( \{ \alpha, \beta \} \) |   |
|  UNTIL | \( \alpha \mathcal{U} \beta \) | \( \{ \beta \} \) | \( \{ \alpha, X(\alpha \mathcal{U} \beta) \} \)  |
|  SINCE | \( \alpha \mathcal{S} \beta \) | \( \{ \beta \} \) | \( \{ \alpha, Y(\alpha \mathcal{S} \beta) \} \)  |
|  RELEASE | \( \alpha \mathcal{R} \beta \) | \( \{ \alpha, \beta \} \) | \( \{ \beta, X(\alpha \mathcal{R} \beta) \} \)  |
|  TRIGGERED | \( \alpha \mathcal{T} \beta \) | \( \{ \alpha, \beta \} \) | \( \{ \beta, Z(\alpha \mathcal{T} \beta) \} \)  |

For ease of exposition, w.l.o.g. we assume formulas to be in NNF. A tableau for a formula  \( \phi \)  is a tree where each node u is labeled by a set of formulas  \( \Gamma(u) \) , with the root  \( u_{0} \)  labeled with  \( \Gamma(u_{0}) = \{\phi\} \) . At each step, a set of rules is applied to a leaf, until all branches have been either accepted or rejected. Each rule either adds one or more children to the current leaf or either accept or reject the current branch. Given a branch  \( \overline{u} = \langle u_{0}, \ldots, u_{n} \rangle \) , the sequence of nodes  \( \langle u_{i}, \ldots, u_{j} \rangle \) , for some  \( 0 \leq i \leq j \leq n \) , is denoted by  \( \overline{u}_{[i,j]} \) .

At each step, the selected node is subject to a number of expansion rules, that select a formula of the label and expand it according to its semantics, as reported in Table 1. Each expansion rule creates one or two children depending on the selected formula. After repeated applications of the expansion rules, a node that only contains elementary formulas, that is, propositions, tomorrow, yesterday, or weak yesterday formulas, is obtained (poised node). Elementary formulas of the form \(\mathsf{X}(\phi_1\mathcal{U}\phi_2)\) are called \(\mathsf{X}\)-eventualities. An \(\mathsf{X}\)-eventuality is a formula that, intuitively, requests something to be fulfilled later. Given an \(\mathsf{X}\)-eventuality \(\phi \equiv \mathsf{X}(\phi_1\mathcal{U}\phi_2)\), \(\phi\) is said to be fulfilled in a node \(u\) if \(\phi_2 \in \Gamma(u)\).

The tableau advances through time by making temporal steps. To do that, the following rules are applied to poised nodes.

STEP A child  \( u_{n+1} \)  is added to  \( u_{n} \) , with:

\[
\Gamma (u _ {n + 1}) = \{\alpha \mid \mathsf {X} \alpha \in \Gamma (u _ {n}) \}
\]

FORECAST Let

\[
G _ {n} = \left\{\alpha \in \mathcal {C} (\phi) \Bigg | \begin{array}{l} \Upsilon \alpha \in \mathcal {C} (\psi) \text {or} \\ Z \alpha \in \mathcal {C} (\psi) \text {for some} \psi \in \Gamma (u _ {n}) \end{array} \right\}
\]

For each subset \( G_{n}^{\prime} \subseteq G_{n} \) (including \( \varnothing \)), a child \( u_{n}^{\prime} \) is added to \( u_{n} \) such that \( \Gamma(u_{n}^{\prime}) = \Gamma(u_{n}) \cup G_{n}^{\prime} \). This is done once and only once before every application of the STEP rule.

The STEP rule advances the construction of the current branch to the subsequent temporal state. The FORECAST is essential to the well-functioning of the rule dealing with past, as it adds a number of branches that nondeterministically guess formulas that may be needed to fulfill past requests coming from future states. For details on the FORECAST rule, we refer the reader to Geatti et al. [13].

Since the STEP rule is not applied to all the poised nodes (to some of which the FORECAST rule is applied instead), we need the following definition.

▶ Definition 2 (Step node). In a complete tableau for an LTL+Past formula, a poised node  \( u_{n} \)  is a step node if it is either a poised leaf or a poised node to which the STEP rule was applied.

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:5

Given a node \( u \), we define \( u^* \) as the closest ancestor of \( u \) that is child of a step node, if any. \( \Gamma^{*}(u) \) is the union of the labels of the nodes from \( u \) to \( u^* \) or to the root, if \( u^* \) does not exist.

Before applying the STEP rule though, poised nodes are subject to the application of a few termination rules, that is, rules that decide whether the construction has to continue or the current branch has to be either rejected or accepted. Given a branch  \( \overline{u} = (u_{0}, \ldots, u_{n}) \) , with  \( u_{n} \)  a step node, the termination rules are the following.

CONTRADICTION If \(\{p, \neg p\} \subseteq \Gamma(u_n)\), for some \(p \in \Sigma\), then \(\overline{u}\) is rejected.

EMPTY If \(\Gamma(u_n) = \varnothing\), then \(\overline{u}\) is accepted.

YESTERDAY If \(\mathsf{Y}\alpha \in \Gamma(u_n)\), then the branch \(\overline{u}\) is rejected if either \(u_n^*\) does not exist or \(Y_{n} \not\subseteq \Gamma^{*}(u_{n}^{*})\), where \(Y_{n} = \{\psi \mid \mathsf{Y}\psi \in \Gamma(u_{n})\}\).

W-YESTERDAY If \( Z\alpha \in \Gamma(u_n) \), then \( \overline{u} \) is rejected if \( u_n^* \) exists and \( Z_n \not\subseteq \Gamma^*(u_n^*) \), where \( Z_n = \{\psi | Z\psi \in \Gamma(u_n)\} \).

LOOP If there exists a position \( i < n \) such that \( \Gamma(u_i) = \Gamma(u_n) \) and all the X-eventualities requested in \( u_i \) are fulfilled in \( \overline{u}_{[i+1\ldots n]} \), then \( \overline{u} \) is accepted.

PRUNE If there exist two positions \(i\) and \(j\) such that \(i < j \leq n\), \(\Gamma(u_i) = \Gamma(u_j) = \Gamma(u_n)\), and all the X-eventualities requested in these nodes which are fulfilled in \(\overline{u}_{[j+1\ldots n]}\) are also fulfilled in \(\overline{u}_{[i+1\ldots j]}\), then \(\overline{u}\) is rejected.

Intuitively, the CONTRADICTION, YESTERDAY, and W-YESTERDAY rules reject branches that contain some contradiction, either a propositional one or because of some unfulfilled past request. The EMPTY rule accepts a branch devoid of contradictions where there is nothing left to do, while the LOOP one accepts a looping branch where all the X-eventualities are proposed again and fulfilled at every repetition of the loop. Finally, the PRUNE rule, which was the main novelty of the system when introduced by Reynolds [22], rejects a branch that, otherwise, is going to be infinitely unrolled because of an X-eventuality impossible to fulfill.

## 3 The translation

In this section, we define a procedure which takes as input an LTL+Past formula, and returns as output an equisatisfiable LTL formula. The increase in size is only linear, thus avoiding the exponential blowup of the worst-case complexity of the translation of an LTL+Past formula into an equivalent (and not simply an equisatisfiable) LTL one [19]. The idea behind the translation is simple, but this is the first time, as far as we know, that it has been actually implemented and experimentally compared with other approaches to support past operators.

The key idea is to replace past subformulas by fresh proposition letters that are forced to replicate the semantics of past operators with ad-hoc axioms. Even though the produced formula is not equivalent to the original one, the proposed translation procedure allows us to easily recover a model of the original formula, if it is satisfiable in the first place, by simply discarding the additional proposition letters. Note that we will define the translation without assuming formulas to be in NNF.

To begin, we define two functions, \(\tau\) and \(\theta\), which are the building blocks of the translation. The function \(\tau\) replaces past formulas with the corresponding placeholder proposition letters, while \(\theta\) enriches the formula with the axioms to force those letters to behave correctly.

Let \(\Sigma\) be the alphabet of proposition letters of the input formula \(\phi\). The alphabet of the output formula is \(\Sigma_{+} = \Sigma \cup \Sigma_{past}\), where \(\Sigma_{past}\) is the set of fresh proposition letters introduced by \(\tau\). We recursively define the function \(\tau: \mathrm{LTL} + \mathrm{Past}[\Sigma] \to \mathrm{LTL}[\Sigma_{+}]\) as follows:

TIME 2021

---

8:6

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

\[
\tau (p) = p \quad \text { where } p \in \Sigma
\]

\[
\tau (\neg \phi) = \neg \tau (\phi)
\]

\[
\tau (\mathsf {X} \phi) = \mathsf {X} \tau (\phi)
\]

\[
\tau (\phi_ {1} \otimes \phi_ {2}) = \tau (\phi_ {1}) \otimes \tau (\phi_ {2}) \quad \text { where } \otimes \in \{\wedge , \vee , \mathcal {U}, \mathcal {R} \}
\]

\[
\tau (\mathsf {Y} \phi) = p _ {\mathsf {Y} \tau (\phi)}
\]

\[
\tau (\mathsf {Z} \phi) = p _ {\mathsf {Z} \tau (\phi)}
\]

\[
\tau (\phi_ {1} \mathcal {S} \phi_ {2}) = p _ {\tau (\phi_ {1}) \mathcal {S} \tau (\phi_ {2})}
\]

\[
\tau (\phi_ {1} \mathcal {T} \phi_ {2}) = \tau (\neg (\neg \phi_ {1} \mathcal {S} \neg \phi_ {2}))
\]

The function \(\Theta : \mathsf{LTL}[\Sigma_{+}] \to 2^{\mathsf{LTL}[\Sigma_{++}]}\), where \(\Sigma_{++} = \Sigma_{+} \cup \{p_{\mathsf{Y}_{p_{\psi}}} \mid p_{\psi} \in \Sigma_{past}, \psi \equiv \phi_1 \mathcal{S} \phi_2\}\), produces a set of formulas which gives the appropriate semantics to the proposition letters in \(\Sigma_{past}\). It is defined as follows:

\[
\Theta (p) = \varnothing \quad \text { where } p \in \Sigma
\]

\[
\Theta (\otimes \phi) = \Theta (\phi) \quad \text { where } \otimes \in \{\neg , X \}
\]

\[
\Theta (\phi_ {1} \otimes \phi_ {2}) = \Theta (\phi_ {1}) \cup \Theta (\phi_ {2}) \quad \text { where } \otimes \in \{\wedge , \vee , \mathcal {U}, \mathcal {R} \}
\]

\[
\Theta (p _ {\mathsf {Y} \phi}) = \{Y _ {p _ {\mathsf {Y} \phi}} \} \cup \Theta (\phi) \quad \text {where} Y _ {p _ {\mathsf {Y} \phi}} \equiv \neg p _ {\mathsf {Y} \phi} \wedge \mathsf {G} (\mathsf {X} p _ {\mathsf {Y} \phi} \leftrightarrow \phi)
\]

\[
\Theta (p _ {\mathsf {Z} \phi}) = \{Z _ {p _ {\mathsf {Z} \phi}} \} \cup \Theta (\phi) \quad \text {where} Z _ {p _ {\mathsf {Z} \phi}} \equiv p _ {\mathsf {Z} \phi} \wedge \mathsf {G} (\mathsf {X} p _ {\mathsf {Z} \phi} \leftrightarrow \phi)
\]

\[
\Theta (p _ {\psi}) = \{S _ {p _ {\psi}}, Y _ {p _ {\mathsf {Y} p _ {\psi}}} \} \cup \Theta (\phi_ {1}) \cup \Theta (\phi_ {2})
\]

where \(\psi \equiv \phi_1\mathcal{S}\phi_2\) and

\[
S _ {p _ {(\phi_ {1} \mathcal {S} \phi_ {2})}} \equiv \mathsf {G} \Big (p _ {(\phi_ {1} \mathcal {S} \phi_ {2})} \leftrightarrow \big (\phi_ {2} \vee \big (\phi_ {1} \wedge p _ {\mathsf {Y} p _ {(\phi_ {1} \mathcal {S} \phi_ {2})}} \big) \Big)
\]

The first three cases are pretty straightforward. The last three, which are the core of the whole translation, are, instead, more involved.

As for the yesterday operator, we state that if the argument  \( \phi \)  of the yesterday operator is true in a certain state, then in the next state  \( Y\phi \)  is true. Note that we force the proposition letter  \( p_{Y\phi} \)  to be false in the initial state, because  \( Y\phi \)  cannot be true in that state. The weak yesterday operator behaves almost the same. However, according to its semantics,  \( Z\phi \)  is always true at the initial state, and thus we constrain the proposition letter  \( p_{Z\phi} \)  to hold at the initial state.

Let us consider now the since operator. By exploiting its semantics and the corresponding expansion rule defined for the tableau in Table 1, we say - in the scope of the always operator - that \(\phi_1\mathcal{S}\phi_2\) is true at a certain state if and only if, at that state, either \(\phi_2\) is true, or \(\phi_1\) is true and \(\phi_1\mathcal{S}\phi_2\) was true at the previous state. However, this is not enough to capture the intended semantics, because we have introduced a new symbol, that is, \(p_{\mathsf{Y}_{p_{\psi}}}\), and we need to force the (yesterday) semantics also for it. This can be done exactly as before.

The function \(\theta : \mathsf{LTL}[\Sigma_{+}] \to \mathsf{LTL}[\Sigma_{++}]\) wraps a formula with the semantics of the additional proposition letters. It is formally defined as follows:

\[
\theta (\phi) = \phi \wedge \bigwedge_ {\psi \in \Theta (\phi)} \psi
\]

The proposed translation procedure is simply the function composition of  \( \theta \)  and  \( \tau \) .

▶ Definition 3 (REMOVEPAST). The function REMOVEPAST: LTL+Past[Σ] → LTL[Σ++] is defined as follows: REMOVEPAST = θ ∘ τ.

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:7

It is possible to prove that the above translation results in an equisatisfiable formula.

Theorem 4. Let \(\phi\) be an LTL+Past formula. Then, REMOVEPAST(\(\phi\)) is an LTL formula equisatisfiable with \(\phi\).

Proof. Since the input formula \(\phi\) is finite, the procedure REMOVEPAST(\(\phi\)) always terminates. We have to show that REMOVEPAST(\(\phi\)) is satisfiable if and only if \(\phi\) is satisfiable.

\((\leftarrow)\) Let \(\overline{\sigma}\) be the model which satisfies \(\phi\), i.e. \(\overline{\sigma} \models \phi\). Let us show that there exists \(\overline{\sigma}'\) such that \(\overline{\sigma}' \models \mathrm{REMOVEPAST}(\phi)\), where, for each \(i \geq 0\) and each sub-formula \(\psi\) of \(\phi\), \(\overline{\sigma}'\) is defined as follows.

1.  \( p \in \sigma_{i}^{\prime} \)  iff  \( \overline{\sigma} \models_{i} p \)  for  \( p \in \Sigma \)
2. \(\tau (\Upsilon \psi)\in \sigma_i^\prime\) iff \(i > 0\) and \(\overline{\sigma}^{\prime}\models_{i - 1}\tau (\psi)\)
3. \(\tau (\mathsf{Z}\psi)\in \sigma_i^\prime\) iff either \(i = 0\) or \(\overline{\sigma}^{\prime}\models_{i - 1}\tau (\psi)\)
4. \(\tau (\psi_1\mathcal{S}\psi_2)\in \sigma_i^\prime\) iff there exists \(0\leq j\leq i\) such that \(\overline{\sigma}^{\prime}\models_{j}\tau (\psi_{2})\) , and \(\overline{\sigma}^{\prime}\models_{k}\tau (\psi_{1})\) for all \(k\) , with \(j <   k\leq i\)

By the definition above, we can prove by induction on the structure of \(\phi\), that \(\overline{\sigma}' \models \tau(\phi)\). Moreover, by Item 2 we have that, for each yesterday sub-formula \(\psi\) of \(\phi\), \(\overline{\sigma}' \models Y_{\tau(\psi)}\), because with Table 2 we force a step-wise consistency between a yesterday formula and its request at the previous state, in \(\overline{\sigma}'\), which is exactly what is stated by \(Y_{\tau(\psi)}\). Similarly, by Item 2 and Item 2, we also have that \(\overline{\sigma}' \models Z_{\tau(\psi)}\) and \(\overline{\sigma}' \models S_{\tau(\psi)}\) for, respectively, each weak yesterday and each since sub-formula \(\psi\) of \(\phi\). This means that \(\overline{\sigma}'\) satisfies the conjunction of the three formulas, hence \(\overline{\sigma}' \models \bigwedge_{\psi \in \Theta(\tau(\phi))} \psi\). Thus, \(\overline{\sigma}' \models \theta(\tau(\phi))\). This allows us to conclude that \(\overline{\sigma}' \models \mathrm{REMOVEPAST}(\phi)\).

\((\rightarrow)\) Given a model \(\overline{\sigma}'\) such that \(\overline{\sigma}' \models \mathrm{REMOVEPAST}(\phi)\), we can easily build \(\overline{\sigma}\) for \(\phi\) by setting that \(p \in \sigma_i\) iff \(\overline{\sigma}' \models_i p\) for all \(p \in \Sigma\). By induction on the structure of \(\phi\), using the semantics of past operators stated by \(Y_{\tau(\psi)}, Z_{\tau(\psi)}, S_{\tau(\psi)}\), we can prove that \(\overline{\sigma} \models \phi\).

## 4 The direct encoding

The BLACK satisfiability checker is based on an iterative procedure that symbolically explores the tableau tree breadth-first by means of a SAT encoding of the tableau branches up to a given depth \( k \), for increasing values of \( k \). The satisfiability checking procedure employed by BLACK is reported in Algorithm 1 [12]. The three formulas \( [\phi]^k \), \( |\phi|^k \), and \( |\phi|^k \) encode different rules of the tableau. This section shows how to extend them to support past operators by encoding the tableau rules recalled in Section 2.

Let us start with some notation. Let \(\phi\) be an LTL+Past formula in NNF over the alphabet \(\Sigma\). We define the following sets of formulas:

\[
\begin{array}{l} \mathrm{XR} = \{\psi \in \mathcal {C} (\phi) \mid \psi \text {   is   a   tomorrow   formula } \} \\ \mathrm{YR} = \{\psi \in \mathcal {C} (\phi) \mid \psi \text {   is   a   yesterday   formula } \} \\ Z R = \{\psi \in \mathcal {C} (\phi) \mid \psi \text {   is   a   weak   yesterday   formula } \} \\ \mathrm{XEV} = \{\psi \in \mathcal {C} (\phi) \mid \psi \text {   is   an   X - eventuality } \} \\ \end{array}
\]

The three encoding formulas are defined over an extended alphabet \(\overline{\Sigma}\), which includes:

1. any proposition letter from the original alphabet \(\Sigma\);
2. the set \(\{p_{\psi} \mid \psi \in \mathsf{XR}, \mathsf{YR}, \mathsf{ZR}\}\), that is, the set of all the grounded X-, Y-, and Z-requests;
3. a stepped version \( p^k \) of all the proposition letters defined in items 1 and 2, with \( k \in \mathbb{N} \) and \( p^0 \) identified as \( p \).

TIME 2021

---

8:8

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

Algorithm 1 BLACK's main procedure [12].

1: procedure BLACK( \( \phi \) )
2:    \( k \leftarrow 0 \) 
3:    while True do
4:    if  \( [\phi]^{k} \)  is UNSAT then
5:    return  \( \phi \)  is UNSAT
6:    end if
7:    if  \( |\phi|^{k} \)  is SAT then
8:    return  \( \phi \)  is SAT
9:    end if
10:    if  \( |\phi|_{T}^{k} \)  is UNSAT then
11:    return  \( \phi \)  is UNSAT
12:    end if
13:    \( k \leftarrow k + 1 \) 
14:    end while
15: end procedure

Intuitively, different stepped versions of the same proposition letter p are used to represent the value of p at different states. Thus, when  \( p^{i} \)  holds, it means that p holds at i-th step node of the branch, i.e. the i-th state of the model.

Moreover, given \(\psi \in \mathcal{C}(\phi)\), we denote by \(\psi_G\) the formula in which all the \(X\)-, \(Y\)-, and \(Z\)-requests are replaced by their grounded version. Similarly, given \(\psi \in \mathcal{C}(\phi)\), we denote by \(\psi^k\) the formula in which all proposition letters are replaced by their \(k\) stepped version. We write \(\psi_G^k\) to denote \((\psi_G)^k\).

The formula \(\llbracket \phi \rrbracket^k\) is called the \(k\)-unraveling of \(\phi\), and encodes the expansion of the tableau tree. To define it, we need to encode the expansion rules of Table 1.

▶ Definition 5 (Stepped Normal Form). Given an LTL+Past formula  \( \phi \)  in NNF, its stepped normal form, denoted by  \( \operatorname{snf}(\phi) \) , is defined as follows:

\(\operatorname{snf}(\ell) = \ell\) where \(\ell \in \{p, \neg p\}\), for \(p \in \Sigma\)  
\(\operatorname{snf}(\otimes \phi_1) = \otimes \phi_1\) where \(\otimes \in \{\mathsf{X}, \mathsf{Y}, \mathsf{Z}\}\)  
\(\operatorname{snf}(\phi_1 \otimes \phi_2) = \operatorname{snf}(\phi_1) \otimes \operatorname{snf}(\phi_2)\) where \(\otimes \in \{\wedge, \vee\}\)  
\(\operatorname{snf}(\phi_1 \mathcal{U} \phi_2) = \operatorname{snf}(\phi_2) \vee (\operatorname{snf}(\phi_1) \wedge \mathsf{X}(\phi_1 \mathcal{U} \phi_2))\)  
\(\operatorname{snf}(\phi_1 \mathcal{R} \phi_2) = \operatorname{snf}(\phi_2) \wedge (\operatorname{snf}(\phi_1) \vee \mathsf{X}(\phi_1 \mathcal{R} \phi_2))\)  
\(\operatorname{snf}(\phi_1 \mathcal{S} \phi_2) = \operatorname{snf}(\phi_2) \vee (\operatorname{snf}(\phi_1) \wedge \mathsf{Y}(\phi_1 \mathcal{S} \phi_2))\)  
\(\operatorname{snf}(\phi_1 \mathcal{T} \phi_2) = \operatorname{snf}(\phi_2) \wedge (\operatorname{snf}(\phi_1) \vee \mathsf{Z}(\phi_1 \mathcal{T} \phi_2))\)

The stepped normal form is the extension to past operators of the next normal form used by Geatti et al. [13]. It can be noted how it follows the expansion rules of each operator in Table 1. We can now define the k-unraveling of  \( \phi \)  recursively as follows:

\(\llbracket \phi \rrbracket^0 = \operatorname{snf}(\phi)_G \wedge \bigwedge_{\psi \in \mathsf{YR}} \neg \psi_G \wedge \bigwedge_{\psi \in \mathsf{ZR}} \psi_G\)  
\(\llbracket \phi \rrbracket^{k+1} = \llbracket \phi \rrbracket^k \wedge S_k \wedge Y_k \wedge Z_k\)  
\(S_k \equiv \bigwedge_{\mathsf{X}\alpha \in \mathsf{XR}} \left((\mathsf{X}\alpha)_G^k \leftrightarrow \operatorname{snf}(\alpha)_G^{k+1}\right)\),

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:9

\[
Y _ {k} \equiv \bigwedge_ {\mathsf {Y} \alpha \in \mathsf {Y R}} \left((\mathsf {Y} \alpha) _ {G} ^ {k + 1} \leftrightarrow \operatorname{snf} (\alpha) _ {G} ^ {k}\right), \qquad Z _ {k} \equiv \bigwedge_ {\mathsf {Z} \alpha \in \mathsf {Z R}} \left((\mathsf {Z} \alpha) _ {G} ^ {k + 1} \leftrightarrow \operatorname{snf} (\alpha) _ {G} ^ {k}\right)
\]

The  \( S_{k} \) ,  \( Y_{k} \)  and  \( Z_{k} \)  formulas encode, respectively, the STEP, YESTERDAY, and W-YESTERDAY rules of the tableau, while the base case of the 0-unraveling ensures that yesterday formulas are false and weak yesterday formulas are true at the first state. The CONTRADICTION rule of the tableau is implicitly encoded in the fact that only satisfying assignments of the formula are considered. Note that the FORECAST rule as well does not need to be explicitly encoded: the intrinsic nondeterminism of the SAT solving process accounts for the nondeterministic choices implemented by the rule.

Intuitively, if \(\llbracket \phi \rrbracket^k\) is unsatisfiable, all the branches of the tableau for \(\phi\) are rejected before \(k + 1\) steps.

▶ Lemma 6. Let \(\phi\) be an LTL+Past formula. Then, \([\phi]^k\) is unsatisfiable if and only if all the branches of the complete tableau for \(\phi\) are crossed by the CONTRADICTION or (W-)YESTERDAY rules and contain at most \(k + 1\) step nodes.

Proof. We prove the contrapositive, i.e. that  \( \left[\phi\right]^{k} \)  is satisfiable if and only if the complete tableau for  \( \phi \)  has at least a branch that is either accepted, crossed by PRUNE, or longer than  \( k+1 \)  step nodes. To do that we establish a connection between truth assignments of  \( \left[\phi\right]^{k} \)  and suitable branches of the tableau.

From branches to assignments. Let \(\overline{u} = \langle u_0, \ldots, u_n \rangle\) be a branch that is either accepted, crossed by PRUNE, or longer than \(k + 1\) step nodes. Let \(\overline{\pi} = \langle \pi_0, \ldots, \pi_m \rangle\) be the sequence of its step nodes. We define a truth assignment \(\nu\) for \([\phi]^k\) as follows. Note that \([\phi]^k\) contains stepped propositions from \(p^0\) until \(p^k\) for any given \(p\), so we need at most \(k + 1\) step nodes from \(\overline{u}\), which however can be shorter if it is accepted or crossed by the PRUNE rule. Hence, let us define \(\ell = \min\{m, k\}\). Moreover, let us define \(p_U\) to be \(p\) if \(p \in \Sigma\), and to be \(\psi\) if \(p = \psi_G\) for some X-, Y-, or Z-request \(\psi\), i.e. \((\cdot)_U\) is the inverse of the \((\cdot)_G\) operation. Then, for \(0 \leq i \leq \ell\), we set \(\nu(p^i) = \top\) if and only if \(p_U \in \Gamma(\pi_i)\). Then, we complete the assignments for positions \(m < j \leq k + 1\) (if any) as follows:

1. if the branch has been accepted by the EMPTY rule, all the other positions \( j > m \) can be filled arbitrarily;
2. if the branch has been accepted by the LOOP or crossed by the PRUNE rule, then there is a position \( w \) such that \( \Gamma(\pi_w) = \Gamma(\pi_m) \). Then we continue filling the truth assignment considering the successor of \( \pi_w \) as a successor of \( \pi_m \).

It can be verified that the truth assignment so constructed satisfies \(\llbracket \phi \rrbracket^k\).

From assignments to branches. Let \(\nu\) be a truth assignment for \([\phi]^k\). We use \(\nu\) as a guide to navigate the tableau tree to find a suitable branch which is either accepted, crossed by PRUNE, or has more than \(k + 1\) step nodes. To do that we build a sequence of branch prefixes \(\overline{u}_i = \langle u_0, \ldots, u_i \rangle\) where at each step we obtain \(\overline{u}_{i+1}\) by choosing \(u_{i+1}\) among the children of \(u_i\), until we find a leaf or we reach \(k + 1\) step nodes. During the descent, we build a partial function \(J: \mathbb{N} \to \mathbb{N}\) that maps positions \(j\) in \(\overline{u}_i\) to indexes \(J(j)\) such that for all \(\psi\) it holds that \(\psi \in \Gamma(u_j)\) if and only if \(\nu \models \operatorname{snf}(\psi)_G^{J(j)}\), i.e. we build a relationship between positions in the branch and steps in \(\nu\). As the base case, we put \(\overline{u}_0 = \langle u_0 \rangle\) and \(J(0) = 0\) so that the invariant holds since \(\Gamma(u_0) = \{\phi\}\) and \(\nu \models \operatorname{snf}(\phi)_G^0\) by the definition of \([\phi]^k\). Then, depending on the rule that was applied to \(u_i\), we choose \(u_{i+1}\) among its children as follows:

1. if the STEP rule has been applied to \( u_{i} \), then there is a unique child that we choose as \( u_{i + 1} \), and we define \( J(i + 1) = J(i) + 1 \). Now, for all \( \mathsf{X}\alpha \in \Gamma (u_i) \), we have \( \alpha \in \Gamma (u_{i + 1}) \) by construction of the tableau. Note that \( \mathrm{snf}(\mathsf{X}\alpha) = \mathsf{X}\alpha \), hence we know by construction that \( \nu \models (\mathsf{X}\alpha)_{\mathsf{G}}^{J(j)} \). Then, by definition of \( [\phi ]^k \), we know that \( \nu \models \mathrm{snf}(\alpha)_{G}^{J(j) + 1} \), i.e.

TIME 2021

---

8:10

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

\(\nu \models \operatorname{snf}(\alpha)_G^{J(i + 1)}\). On the other direction, if \(\nu \models \operatorname{snf}(\alpha)_G^{J(i + 1)}\), then by definition of \([\phi]^k\) we have \(\nu \models (\mathsf{X}\alpha)_G^{J(i)}\), hence \(\nu \models \operatorname{snf}(\mathsf{X}\alpha)_G^{J(i)}\), hence \(\mathsf{X}\alpha \in \Gamma(u_i)\), so by construction of the tableau we have \(\alpha \in \Gamma(u_{i + 1})\). Hence the invariant holds.

2. if the FORECAST rule has been applied to \( u_{i} \), then there are \( n \) children \( \{u_i^1, \ldots, u_i^n\} \) such that \( \Gamma(u_i) \subseteq \Gamma(u_i^m) \) for all \( 1 \leq m \leq n \). Now, we set \( J(i + 1) = J(i) \) and we choose \( u_{i+1} \) as a child \( u_i^m \) with a label \( \Gamma(u_i^m) \) such that for any \( \psi \) we have \( \psi \in \Gamma(u_{i+1}) \) if and only if \( \nu \models \operatorname{snf}(\psi)_G^{J(i+1)} \). Note that at least one such child exists, because at least one child has the same label as \( u_i \). Thus the invariant holds by construction.
3. if an expansion rule has been applied to \( u_{i} \), then there are one or two children. In both cases, we set \( J(i + 1) = J(i) \). Then:

a. if there is one child, then it is chosen as \( u_{i+1} \). In this case, the rule is the CONJUNCTION rule and has been applied to a formula \( \psi \equiv \psi_1 \wedge \psi_2 \), hence \( \psi_1, \psi_2 \in \Gamma(u_{i+1}) \). By construction we know that \( \nu \models \operatorname{snf}(\psi)_G^{J(i)} \), hence \( \nu \models \operatorname{snf}(\psi)_G^{J(i+1)} \). Now, note that \( \operatorname{snf}(\psi_1 \wedge \psi_2) = \operatorname{snf}(\psi_1) \wedge \operatorname{snf}(\psi_2) \), so it holds that \( \nu \models \operatorname{snf}(\psi_1)_G^{J(i+1)} \) and \( \nu \models \operatorname{snf}(\psi_2)_G^{J(i+1)} \). On the other direction, if \( \nu \models \operatorname{snf}(\psi_1)_G^{J(i+1)} \) and \( \nu \models \operatorname{snf}(\psi_2)_G^{J(i+1)} \) we know that \( \nu \models \operatorname{snf}(\psi_1 \wedge \psi_2)_G^{J(i+1)} \) hence \( \nu \models \operatorname{snf}(\psi_1 \wedge \psi_2)_G^{J(i)} \), hence by construction we have \( \psi_1 \wedge \psi_2 \in \Gamma(u_i) \) and so we have \( \psi_1, \psi_2 \in \Gamma(u_i) \), hence the invariant holds.

b. if there are two children \( u_i' \) and \( u_i'' \), then let us suppose the rule applied is the DISJUNCTION rule. Similar arguments will hold for the other rules. In this case, the rule has been applied to a formula \( \psi \equiv \psi_1 \vee \psi_2 \), hence \( \psi_1 \in \Gamma(u_i') \) and \( \psi_2 \in \Gamma(u_i'') \). We know \( \nu \models \operatorname{snf}(\psi)_G^{J(i)} \), hence \( \nu \models \operatorname{snf}(\psi)_G^{J(i+1)} \). Since \( \operatorname{snf}(\psi_1 \vee \psi_2) = \operatorname{snf}(\psi_1) \vee \operatorname{snf}(\psi_2) \), it holds that either \( \nu \models \operatorname{snf}(\psi_1)_G^{J(i+1)} \) or \( \nu \models \operatorname{snf}(\psi_2)_G^{J(i+1)} \). Now, we choose \( u_{i+1} \) accordingly, so to respect the invariant. Note that if both nodes are eligible, which one is chosen does not matter. The other direction of the invariant also holds, since if either \( \nu \models \operatorname{snf}(\psi_1)_G^{J(i+1)} \) or \( \nu \models \operatorname{snf}(\psi_2)_G^{J(i+1)} \), then \( \nu \models \operatorname{snf}(\psi_1)_G^{J(i)} \) or \( \nu \models \operatorname{snf}(\psi_2)_G^{J(i)} \), so \( \nu \models \operatorname{snf}(\psi_1 \vee \psi_2)_G^{J(i)} \), hence \( \psi_1 \vee \psi_2 \in \Gamma(u_i) \), hence either \( \psi_1 \in \Gamma(u_{i+1}) \) or \( \psi_2 \in \Gamma(u_{i+1}) \).

Let \(\overline{u} = \langle u_0, \ldots, u_i \rangle\) be the branch prefix constructed as above, and let \(\overline{\pi} = \langle \pi_0, \ldots, \pi_n \rangle\) be the sequence of its step nodes. As mentioned, the descent stops when \(\pi_n\) is a leaf or when \(n = k + 1\). Note in any case that \(u_i = \pi_n\). In case we find a leaf, note that it cannot have been crossed by the CONTRADICTION rule. Otherwise, we would have \(\{p, \neg p\} \subseteq \Gamma(u_i)\), which would mean \(\nu \models p^{J(i)}\) and \(\nu \models \neg p^{J(i)}\), which is not possible. Moreover, it cannot have been crossed by the YESTERDAY rule, since that would mean there is some \(\Upsilon\alpha \in \Gamma(\pi_n)\) with \(\alpha \notin \Gamma^*(\pi_{n-1})\). But, we know that \(\nu \models \operatorname{snf}(\Upsilon\alpha)_G^{J(i)}\), hence \(\nu \models (\Upsilon\alpha)_G^{J(i)}\) since \(\operatorname{snf}(\Upsilon\alpha) = \Upsilon\alpha\). Then, by definition of \([\phi]^k\), we know that \(\nu \models \operatorname{snf}(\alpha)_G^{J(i)-1}\). Since \(u_i = \pi_n\) is a step node, \(J(i) - 1 = J(j)\) for some \(j\) such that \(u_j = \pi_{n-1}\), hence \(\nu \models \operatorname{snf}(\alpha)_G^{J(j)}\), and by construction we know that \(\alpha \in \Gamma(u_j)\), which conflicts with the hypothesis that the YESTERDAY rule crossed the branch. With a similar argument, we can see that it cannot have been crossed by the W-YESTERDAY rule neither. Hence we found a branch which is either longer than \(k + 1\) step nodes, or have been accepted, or have been crossed by the PRUNE rule.

The formula  \( |\phi|^{k} \)  is called the base encoding of  \( \phi \)  and, in addition to the k-unraveling, includes the encoding of the EMPTY and LOOP rules, i.e. the rules that can accept branches. The formula is defined as:

\[
| \phi | ^ {k} \equiv [ [ \phi ] ] ^ {k} \wedge (E _ {k} \vee L _ {k})
\]

where the  \( E_{k} \)  formula encodes the EMPTY rule and is defined as follows:

\[
E _ {k} \equiv \bigwedge_ {\psi \in \mathsf {X R}} \neg \psi_ {G} ^ {k}
\]

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:11

and the \(L_{k}\) formula encodes the LOOP rule and is defined as:

\[
L _ {k} \equiv \bigvee_ {l = 0} ^ {k - 1} \left(_ {l} R _ {k} \wedge {} _ {l} F _ {k}\right)
\]

where

\[
{ } _ { l } R _ { k } \equiv \bigwedge _ { \psi \in \mathsf { X R } \cup \mathsf { Y R } \cup \mathsf { Z R } } \left( \psi _ { G } ^ { l } \leftrightarrow \psi _ { G } ^ { k } \right) , \text {~ a~ n~ d~ }
\]

\[
{ } _ { l } F _ { k } \equiv \bigwedge _ { \psi \in \mathsf { X E V } \atop \psi \equiv \mathsf { X } ( \psi _ { 1 } \mathcal { U } \psi _ { 2 } ) } \left( \psi _ { G } ^ { k } \to \bigvee _ { i = l + 1 } ^ { k } \operatorname{snf} ( \psi _ { 2 } ) _ { G } ^ { i } \right) .
\]

Intuitively,  \( _{l}R_{k} \)  encodes the presence of two nodes whose labels contain the same requests for the next and the previous nodes, while  \( _{l}F_{k} \)  checks that all the X-eventualities are fulfilled between those nodes. It can be proved that  \( |\phi|^{k} \)  correctly encodes tableau trees where at least one branch is accepted in  \( k+1 \)  steps.

▶ Lemma 7. Let  \( \phi \)  be an LTL+Past formula. If the complete tableau for  \( \phi \)  contains an accepted branch of  \( k+1 \)  step nodes, then  \( |\phi|^{k} \)  is satisfiable.

Proof. Suppose that the complete tableau for \(\phi\) contains an accepted branch of \(k + 1\) step nodes, so let \(\overline{u} = \langle u_0, \ldots, u_n \rangle\) be such a branch, and let \(\overline{\pi} = \langle \pi_0, \ldots, \pi_k \rangle\) be the sequence of its step nodes. Then, by Lemma 6, \([\phi]^k\) is satisfiable. We can then build a truth assignment \(\nu\) in the same way as in the proof of Lemma 6, such that \(\nu \models [\phi]^k\). Remember that this means we set \(\nu(p^i) = \top\) if and only if \(p_U \in \Gamma(\pi_i)\) for all \(0 \leq i \leq k\). So now we have to prove that \(\nu\) satisfies either \(E_k\) or \(L_k\). We will need an auxiliary fact, that is, that \(\psi \in \Gamma^*(\pi_i)\) if and only if \(\nu \models \operatorname{snf}(\psi)_G^i\). That can be done by induction on the structure of \(\psi\), exploiting the definition of the expansion rules of the tableau.

Now, we distinguish two cases depending on which rule accepted the branch:

1. if the branch was accepted by the EMPTY rule, then \(\Gamma(\pi_k) = \varnothing\), hence, in particular \(\Gamma(\pi_k)\) does not contain any X-request. Hence by definition of \(\nu\) we have that \(\nu \models \neg \psi_G^k\) for any \(\psi \in \mathsf{XR}\), so \(E_k\) is satisfied;

2. if the branch was accepted by the LOOP rule, then we have a node \(\pi_l\) such that \(\Gamma(\pi_l) = \Gamma(\pi_k)\), hence by definition of \(\nu\) we have \(\nu \models \psi_G^l\) if and only if \(\nu \models \psi_G^k\) for any \(\psi \in \mathsf{XR} \cup \mathsf{YR} \cup \mathsf{ZR}\), so \(lR_k\) is satisfied. Moreover, we know that for any X-eventuality \(\psi \equiv \mathsf{X}(\psi_1\mathcal{U}\psi_2)\) requested in \(\Gamma(\pi_k)\), \(\psi\) has been fulfilled between \(\pi_l\) and \(\pi_k\), i.e. there is a \(l < j \leq k\) such that \(\psi_2 \in \Gamma^*(\pi_j)\). Hence we know that \(\nu \models \operatorname{snf}(\psi_2)_G^j\), hence \(lF_k\) is satisfied. Then, \(lR_k \wedge_l F_k\) is satisfied for at least one \(l\), so \(L_k\) is satisfied.

▶ Lemma 8. Let  \( \phi \)  be an LTL+Past formula. If  \( |\phi|^{k} \)  is satisfiable then the complete tableau for  \( \phi \)  contains an accepted branch.

Proof. Suppose that \( |\phi|^k \) is satisfiable, hence we have a truth assignment \( \nu \) such that \( \nu \models |\phi|^k \). Then, \( [\phi]^k \) is satisfiable, and we know from Lemma 6 that the complete tableau for \( \phi \) has a branch that is either accepted, crossed by PRUNE, or longer than \( k + 1 \) step nodes. Let \( \overline{u} = \langle u_0, \ldots, u_n \rangle \) be the branch prefix found as shown in the proof of Lemma 6, and let \( \overline{\pi} = \langle \pi_0, \ldots, \pi_m \rangle \) be the sequence of its step nodes. By construction we have a function \( J: \mathbb{N} \to \mathbb{N} \) fulfilling the invariant that \( \psi \in \Gamma(u_i) \) if and only if \( \nu \models \operatorname{snf}(\psi)_G^{J(i)} \). We now show that indeed \( \overline{u} \) is accepted or is the prefix of an accepted branch. Since \( |\phi|^k \) is satisfiable, either \( E_k \) or \( L_k \) are satisfiable as well:

TIME 2021

---

8:12

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

1. if \( E_{k} \) is satisfiable, we know that \( \nu \models \neg \psi_G^k \) for each \( \psi \in \mathsf{XR} \). Since \( \psi \) is an X-request, \( \operatorname{snf}(\psi) \equiv \psi \), so \( \nu \not\models \operatorname{snf}(\psi)_G^k \). Here, \( k = J(j) \) for some \( j \), and from the invariant we know that \( \psi \not\in \Gamma(u_j) \). Hence, \( u_j \) does not contain any X-request, so its successor \( u_{j+1} \) has an empty label, triggering the EMPTY rule that accepts the branch.
2. if \( L_{k} \) is satisfiable, so are \( _{l}R_{k} \) and \( _{l}F_{k} \) for some \( 0 \leq l < k \). Hence from \( _{l}R_{k} \) we know that \( \nu \models \psi_{G}^{l} \) if and only if \( \nu \models \psi_{G}^{k} \) for all \( \psi \in \mathsf{XR} \cup \mathsf{YR} \cup \mathsf{ZR} \), that is \( \nu \models \operatorname{snf}(\psi)_{G}^{l} \) if and only if \( \nu \models \operatorname{snf}(\psi)_{G}^{k} \) because \( \psi \) is an X-, Y-, or Z-request. Here, \( l = J(i) \) and \( k = J(j) \) for some \( i \) and some \( j \). Since the value of the function \( J \) increments at each step node, we can assume w.l.o.g. that \( u_{i} \) and \( u_{j} \) are step nodes, and by the invariant we know \( \psi \in \Gamma(u_{i}) \) if and only if \( \psi \in \Gamma(u_{j}) \), i.e. \( u_{i} \) and \( u_{j} \) have the same X-, Y-, and Z-request. Similarly, the fact that \( \nu \models _{l}F_{k} \) tells us that all the X-eventualities requested in \( u_{i} \) are fulfilled between \( u_{i+1} \) and \( u_{j} \). The LOOP rule requires two identical labels in order to trigger, but \( u_{i} \) and \( u_{j} \) only have the same requests. However, since they have the same X-requests, we know that \( \Gamma(u_{i+1}) = \Gamma(u_{j+1}) \). Then, there is a step node \( u_{j'} \), grandchild of \( u_{j} \), such that \( \Gamma(u_{j}) = \Gamma(u_{j'}) \) and the segment of the branch between \( u_{i+1} \) and \( u_{j} \) is equal to the segment between \( u_{j+1} \) and \( u_{j'} \), hence all the X-eventualities requested in \( u_{i} \) and \( u_{j} \), fulfilled between \( u_{i+1} \) and \( u_{j} \), are fulfilled between \( u_{j+1} \) and \( u_{j'} \) as well, and the LOOP rule can apply to \( u_{j'} \), accepting the branch.

Lastly, the formula \( |\phi|_T^k \), called the termination encoding, encodes the PRUNE rule. The formula is defined as follows:

\[
| \phi | _ {T} ^ {k} \equiv [   [ \phi ]   ] ^ {k} \wedge \bigwedge_ {i = 0} ^ {k} \neg P ^ {i}
\]

where

\[
P ^ {k} \equiv \bigvee_ {l = 0} ^ {k - 2} \bigvee_ {j = l + 1} ^ {k - 1} \left(_ {l} R _ {j} \wedge {} _ {j} R _ {k} \wedge {} _ {l} P _ {j} ^ {k}\right)
\]

\[
{ } _ { l } P _ { j } ^ { k } \equiv \bigwedge _ { \psi \in \mathsf { X E V } \atop \psi \equiv \mathsf { X } ( \psi _ { 1 } \mathcal { U } \psi _ { 2 } ) } \left( \psi _ { G } ^ { k } \wedge \bigvee _ { i = j + 1 } ^ { k } \operatorname{snf} ( \psi _ { 2 } ) _ { G } ^ { i } \to \bigvee _ { i = l + 1 } ^ { j } \operatorname{snf} ( \psi _ { 2 } ) _ { G } ^ { i } \right)
\]

It can be proved that  \( |\phi|_{T}^{k} \)  is unsatisfiable if the tableau for  \( \phi \)  contains only rejected branches.

▶ Lemma 9. Let  \( \phi \)  be an LTL+Past formula. If  \( |\phi|_{T}^{k} \)  is unsatisfiable, then the complete tableau for  \( \phi \)  contains only rejected branches.

Proof. We prove the contrapositive, i.e. that if the complete tableau for \(\phi\) contains an accepted branch, then \(|\phi|_T^k\) is satisfiable. Let \(\overline{u} = \langle u_0, \ldots, u_n \rangle\) be such a branch, and let \(\overline{\pi} = \langle \pi_0, \ldots, \pi_m \rangle\) be the sequence of its step nodes. By Lemma 6, we know \([\phi]^k\) is satisfiable, thus we can obtain a truth assignment \(\nu\) such that \(\nu \models [\phi]^k\). We can build \(\nu\) as in the proof of Lemma 6, i.e. such that \(\nu(p^i) = \top\) if and only if \(p_U \in \Gamma(\pi_i)\) for all \(0 \leq i \leq k\). Similarly to the proof of Lemma 7, we highlight the fact that \(\psi \in \Gamma^*(\pi_i)\) if and only if \(\nu \models \operatorname{snf}(\psi)_G^i\). Now, since the branch is accepted, the PRUNE rule cannot be applied to it. This means that either a) there are no three nodes \(\pi_u\), \(\pi_v\), \(\pi_w\) such that \(\Gamma(\pi_u) = \Gamma(\pi_v) = \Gamma(\pi_w)\), or b) these three nodes exist but there is an X-eventuality \(\psi\) requested in \(\Gamma(\pi_w)\) that is fulfilled between \(\pi_u\) and \(\pi_v\) and not between \(\pi_v\) and \(\pi_w\). In case a) this means \(uR_v \wedge vR_w\) does not hold for any \(u\) and \(v\). In case b), \(uR_v \wedge vR_w\) holds but \(uP_v^w\) does not. In any case, it follows that \(\neg P^i\) holds for any \(0 \leq i \leq k\), hence \(|\phi|_T^k\) is satisfied.

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:13

Together with the soundness and completeness results for the underlying tableau [13], the above Lemmata allow us to prove the soundness and completeness of the procedure of Algorithm 1.

Theorem 10 (Soundness and completeness). Let \(\phi\) be an LTL+Past formula. The BLACK algorithm answers satisfiable on \(\phi\) if and only if \(\phi\) is satisfiable.

Proof. ( \( \rightarrow \) ) Suppose the BLACK algorithm answers satisfiable on the formula  \( \phi \) . Then, it means there is a  \( k \geq 0 \)  such that  \( |\phi|^{k} \)  is satisfiable. By Lemma 8, the complete tableau for  \( \phi \)  has an accepting branch. By the soundness of the tableau, then  \( \phi \)  is satisfiable.

\((\leftarrow)\) Now suppose the formula \(\phi\) is satisfiable. By the completeness of the tableau, the complete tableau for \(\phi\) has an accepting branch. Let us suppose such a branch has \(k + 1\) step nodes for some \(k \geq 0\). Then, we want to show that the BLACK algorithm eventually answers satisfiable. Let \(i < k\) be any earlier iteration of the main loop of the algorithm. We have that by Lemma 6, \([\phi]^i\) is satisfiable because there is a branch longer than \(i + 1\) step nodes. Similarly, by Lemma 9, \(|\phi|_T^i\) is satisfiable because not all the branches of the tableau are rejected. Hence, the algorithm does not answer unsatisfiable at step \(i\). Arrived at step \(k\), \(|\phi|^k\) is satisfiable by Lemma 7 because the tableau has an accepted branch of \(k + 1\) step nodes, hence the algorithm answers satisfiable.

Despite the final encoding may seem trivially simple, this simplicity makes the approach interesting for two reasons. First of all, it was not a priori clear that such a simple encoding would be possible, given the presence of the FORECAST rule: a rule that kills the performance of the explicit construction of the tableau turns out to be a non-problem in the symbolic exploration of the tree. Secondly, this simplicity is what drives the experimental success of the encoding. Comparing it for example with Biere et al. [2] which is, as far as we know, the approach implemented by nuXmv: all the virtual unrollings machinery that they need to support past operators is much more complex than our seemingly trivial encoding which, however, performs better even though it does not generate CNF formulas of linear size. Hence, showing that the approach by Geatti et al. [12] can be successfully extended so easily can be considered one of the main contributions of this work.

## 5 Experimental results

We have implemented the above two approaches in version 0.3.0 of the BLACK tool \( ^{1} \) : the translation as an optional module that can be activated upon user request, and the direct encoding as an expansion of the core procedure.

Since the literature lacks significant LTL+Past family of formulas for benchmarks, we have devised two novel sets of formulas. As for the first family, we chose a set of random formulas, generated with an algorithm adapted from [24], in order to verify how the tool scales in general. The second family, that we called crscounter, is inspired by and adapted from Cimatti et al. [7], where a Kripke structure called Counter(N), where N is a power of two, is introduced. Counter(N) works as follows: it starts at c = 0, counts up to c = N, jumps back to c = N/2, and then loops, counting up to c = N and jumping back to c = N/2, forever. Afterwards, they evaluated, on top of that Kripke structure, some parametric properties of the form:

\[
P (i) \equiv \neg \mathsf {F} \big (\mathsf {O} ((c = \frac {N}{2}) \land \mathsf {O} ((c = \frac {N}{2} + 1) \land \ldots \land \mathsf {O} (c = \frac {N}{2} + i) \ldots)) \big).
\]

\( ^{1} \)  The tool can be found at https://github.com/black-sat/black. Packages for macOS and common Linux distributions are provided, together with all the necessary scripts to reproduce the tests performed.

TIME 2021

---

8:14

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

![img-0.jpeg](img-0.jpeg)

Figure 1 Experimental results of BLACK against the other tested tools and modalities, for random formulas of size  \( n \in \{500, 1000, 2000, 3000, 5000, 10000\} \) . Times are in seconds. Note that only instances that could not be solved by any tool are marked as unsolved.

The value i identifies the number of nested once operators, while the structure of such properties requires that the loop of length N/2 in the model is traversed backwards several times in order to reach a counterexample.

The crscounter benchmarks were introduced in the context of model checking. Thus, we made a reduction from the model checking problem to the satisfiability checking one for LTL+Past: we built the LTL+Past formulas  \( \phi_{Counter(N)} \)  and  \( \phi_{P(i)} \)  encoding the above elements. In this way,  \( \neg(\phi_{Counter(N)} \to \phi_{P(i)}) \)  is UNSAT if and only if  \( Counter(N) \models P(i) \) . With this framework, we were able to obtain both SAT ( \( i \leq \frac{N}{2} \) ) and UNSAT ( \( i > \frac{N}{2} \) ) instances. Moreover, this family of formulas stresses the ability to process past operators and find short counterexamples, and thus, it specifically challenges our contribution.

For each formula, we executed both an internal comparison between the two proposed techniques – with BLACK over MathSAT [4], which is the best performing solver among those supported by BLACK so far –, and an external comparison with nuXmv [5], which, as far as we know, is the only state-of-the-art tool directly supporting past operators. Specifically, we tested BLACK against nuXmv in both sbmc and klive modalities. The former stands for Simple Bounded Model Checking [2], and it is the closest to our approach between the two. The latter has been proposed more recently, and it is based on K-Liveness [6].

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:15

![img-1.jpeg](img-1.jpeg)

Figure 2 Experimental results for crscounter formulas of size \( N \in \{8, 16, 32\} \). Green vertical bars indicate where formulas start to be UNSAT.

All the experiments have been performed on a Dell PowerEdge rack server equipped with a 16-cores AMD EPYC \( ^{™} \) 7281 CPU (2.7GHz) and 64GB of RAM (DDR4 2400 MT/s). Moreover, experiments have been run in parallel, each on a single core, with a memory limit of 3GB of RAM per core, and a 10 minutes timeout. Experimental results plots can be found in Figure 1 and Figure 2.

Regarding the random formulas, what immediately catches the eye is that BLACK- in the direct approach - performs overall significantly better, in particular if compared with nuXmv. Indeed, the latter starts to be in difficulty already at size \( n = 3000 \), and gets almost always stuck at size \( n = 10000 \). It can also be noticed that BLACK solves UNSAT instances quicker than the SAT ones. This could be a bit surprising, considering that the former is a universal property, while the latter is an existential one. However, this has a twofold explanation. Firstly, the Boolean encoding acts like a breadth-first search, which is in some sense an exhaustive search, up to depth \( k \). Secondly, in all UNSAT random formulas the tableaux are closed by contradiction, and never by the PRUNE rule. This is because of the nature of the random formulas generator: it is less likely to generate a formula with such a peculiar structure as to trigger the PRUNE rule. Also, it is more likely to produce contradictions that can be spotted in the first depth levels. Nevertheless, it is interesting to note that BLACK is able to manage those UNSAT formulas better than nuXmv, overall.

Looking instead at the crscounter plots, there are two interesting aspects to point out. First, BLACK performs slightly better with the direct past encoding than with the translation approach. Second, while BLACK performs worse with SAT formulas, the situation overturns

TIME 2021

---

8:16

Past Matters: Supporting LTL+Past in the BLACK Satisfiability Checker

when the formulas become UNSAT. This can be explained by the PRUNE rule: it may cause an overhead in the encoding of SAT formulas, but it has the advantage of guaranteeing a quicker termination, i.e. closure of the tableau, in case of UNSAT ones. Indeed, in this family of formulas, all UNSAT ones have a tableau which is closed by the PRUNE rule.

## 6 Conclusions

This work contributes to the state of the art of the satisfiability problem for LTL+Past in different directions. First of all, it provides a translation procedure for LTL+Past, that has been implemented into the BLACK tool, but, in principle, can be used also as a preprocessing step for any LTL satisfiability checker in order to let them support the past. Then, it extends the SAT-based encoding of a one-pass and tree-shaped tableau for LTL to LTL+Past, and shows that the resulting tool is very effective and efficient compared to the state-of-the-art ones. It is also worth noting that the encoding is quite simple compared to the virtual unrollings techniques used to support past operators by Latvala et al. [15], and it offers an exponential advantage over the explicit construction of the tableau since the FORECAST rule is not required to be encoded. Finally, it introduces two new sets of formulas which aim at starting to build a larger set of standardized formulas for benchmarks in the field.

Results showed that the direct encoding is the preferred way to manage the past, but also that the translation is a viable alternative, as it adds only a linear overhead.

Future work should head at reducing the overhead introduced by the PRUNE rule, possibly by triggering it not at each step, but with some predetermined heuristics, in order to reduce the gap in performance between SAT and UNSAT properties. Moreover, since the framework has been shown to be efficient and modular, it could be interesting to investigate its extension to other logics. Orthogonally, some in-depth theoretical comparison with other state-of-the-art techniques could suggest new ways of improvement.

## References

1 Matteo Bertello, Nicola Gigante, Angelo Montanari, and Mark Reynolds. Leviathan: A new LTL satisfiability checking tool based on a one-pass tree-shaped tableau. In Proc. of the 25th International Joint Conference on Artificial Intelligence, pages 950–956. IJCAI/AAAI Press, 2016. URL: http://www.ijcai.org/Abstract/16/139.

2 Armin Biere, Keijo Heljanko, Tommi A. Junttila, Timo Latvala, and Viktor Schuppan. Linear encodings of bounded LTL model checking. Log. Methods Comput. Sci., 2(5), 2006. doi:10.2168/LMCS-2(5:5)2006.

3 Laura Bozzelli, Aniello Murano, and Loredana Sorrentino. Alternating-time temporal logics with linear past. Theor. Comput. Sci., 813:199–217, 2020. doi:10.1016/j.tcs.2019.11.028.

4 Roberto Bruttomesso, Alessandro Cimatti, Anders Franzén, Alberto Griggio, and Roberto Sebastiani. The MathSAT 4 SMT Solver. In Computer Aided Verification, LNCS, pages 299–303. Springer, 2008. doi:10.1007/978-3-540-70545-1_28.

5 Roberto Cavada, Alessandro Cimatti, Michele Dorigatti, Alberto Griggio, Alessandro Mariotti, Andrea Micheli, Sergio Mover, Marco Roveri, and Stefano Tonetta. The nuXmv Symbolic Model Checker. In Computer Aided Verification, pages 334–342. Springer, 2014. doi:10.1007/978-3-319-08867-9_22.

6 Alessandro Cimatti, Alberto Griggio, Sergio Mover, and Stefano Tonetta. Verifying LTL Properties of Hybrid Systems with K-Liveness. In Computer Aided Verification, LNCS, pages 424–440. Springer, 2014. doi:10.1007/978-3-319-08867-9_28.

7 Alessandro Cimatti, Marco Roveri, and Daniel Sheridan. Bounded Verification of Past LTL. In Formal Methods in Computer-Aided Design, LNCS, pages 245–259. Springer, 2004. doi:10.1007/978-3-540-30494-4_18.

8 Edmund M. Clarke, Thomas A. Henzinger, Helmut Veith, and Roderick Bloem, editors. Handbook of Model Checking. Springer, 2018. doi:10.1007/978-3-319-10575-8.

---

L. Geatti, N. Gigante, A. Montanari, and G. Venturato

8:17

9 Michael Fisher. A resolution method for temporal logic. In John Mylopoulos and Raymond Reiter, editors, Proceedings of the 12th International Joint Conference on Artificial Intelligence, pages 99–104. Morgan Kaufmann, 1991.

10 Michael Fisher. A normal form for temporal logics and its applications in theorem-proving and execution. J. Log. Comput., 7(4):429–456, 1997. doi:10.1093/logcom/7.4.429.

11 Michael Fisher, Dov M. Gabbay, and Lluís Vila, editors. Handbook of Temporal Reasoning in Artificial Intelligence, volume 1 of Foundations of Artificial Intelligence. Elsevier, 2005. URL: http://cgi.csc.liv.ac.uk/%7Emichael/handbook.html.

12 Luca Geatti, Nicola Gigante, and Angelo Montanari. A SAT-Based Encoding of the One-Pass and Tree-Shaped Tableau System for LTL. In Proc. of the 28th International Conference on Automated Reasoning with Analytic Tableaux and Related Methods, volume 11714 of LNCS, pages 3–20. Springer, 2019. doi:10.1007/978-3-030-29026-9_1.

13 Luca Geatti, Nicola Gigante, Angelo Montanari, and Mark Reynolds. One-pass and tree-shaped tableau systems for TPTL and TPTL₀+Past. Information and Computation, 2021. in press. doi:10.1016/j.ic.2020.104599.

14 Ullrich Hustadt and Boris Konev. TRP++2.0: A Temporal Resolution Prover. In Proc. of the 19th International Conference on Automated Deduction, pages 274–278, 2003.

15 Timo Latvala, Armin Biere, Keijo Heljanko, and Tommi A. Juntila. Simple is better: Efficient bounded model checking for past LTL. In Proc. of the 6th International Conference on Verification, Model Checking, and Abstract Interpretation, volume 3385 of LNCS, pages 380–395. Springer, 2005. doi:10.1007/978-3-540-30579-8_25.

16 Jianwen Li, Yinbo Yao, Geguang Pu, Lijun Zhang, and Jifeng He. Aalta: an LTL satisfiability checker over infinite/finite traces. In Proc. of the 22nd ACM International Symposium on Foundations of Software Engineering, pages 731–734. ACM, 2014. doi:10.1145/2635868.2661669.

17 Orna Lichtenstein and Amir Pnueli. Propositional Temporal Logics: Decidability and Completeness. Logic Journal of the IGPL, 8(1):55–85, 2000. doi:10.1093/jigpal/8.1.55.

18 Orna Lichtenstein, Amir Pnueli, and Lenore D. Zuck. The Glory of the Past. In Proc. of the 1st Conference on Logics of Programs, volume 193 of LNCS, pages 196–218. Springer, 1985. doi:10.1007/3-540-15648-8_16.

19 Nicolas Markey. Temporal logic with past is exponentially more succinct. Bulletin of the EATCS, 79:122–128, 2003.

20 Amir Pnueli. The Temporal Logic of Programs. In Proc. of the 18th Annual Symposium on Foundations of Computer Science, pages 46–57. IEEE Computer Society, 1977. doi:10.1109/SFCS.1977.32.

21 Mark Reynolds. More Past Glories. In Proc. of the 15th Annual IEEE Symposium on Logic in Computer Science, pages 229–240. IEEE Computer Society, 2000. doi:10.1109/LICS.2000.855772.

22 Mark Reynolds. A New Rule for LTL Tableaux. In Proc. of the 7th International Symposium on Games, Automata, Logics and Formal Verification, volume 226 of EPTCS, pages 287–301, 2016. doi:10.4204/EPTCS.226.20.

23 A. Prasad Sistla and Edmund M. Clarke. The Complexity of Propositional Linear Temporal Logics. Journal of the ACM, 32(3):733–749, 1985. doi:10.1145/3828.3837.

24 Heikki Tauriainen and Keijo Heljanko. Testing LTL formula translation into Büchi automata. International Journal on Software Tools for Technology Transfer, 4(1):57–70, 2002. doi:10.1007/s100090200070.

25 G. S. Tseitin. On the Complexity of Derivation in Propositional Calculus. In Jörg H. Siekmann and Graham Wrightson, editors, Automation of Reasoning: 2: Classical Papers on Computational Logic 1967–1970, Symbolic Computation, pages 466–483. Springer, Berlin, Heidelberg, 1983. doi:10.1007/978-3-642-81955-1_28.

26 Pierre Wolper. Temporal Logic Can Be More Expressive. Information and Control, 56(1/2):72–99, 1983. doi:10.1016/S0019-9958(83)80051-5.

TIME 2021