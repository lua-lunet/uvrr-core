# Reasoning About Programs: Solving Two Problems Using Programming

Source: https://youtu.be/GX3URhx6i2E
Author: E. W. Dijkstra, introduced by E. Schmidt (Sun Microsystems). Uploader "AJT";
uploaded 2016-04-19; runtime 54:49.
Fetched: 2026-10-02 via `yt-dlp --write-auto-subs --sub-langs en.*` (the `en-orig`
caption track, de-rolled) and via the Mistral transcription endpoint
(`POST https://api.mistral.ai/v1/audio/transcriptions`, model `voxtral-mini-latest`,
`language=en`, `timestamp_granularities[]=segment`; 402 segments over 3257 s of
audio). Board content read from frames at 00:07:08-00:07:30.

---

## What this record is, and is not

This is an **edited synthesis**, not a transcript. Two independent automatic
speech recognitions of the same audio — YouTube's own caption track and a
Mistral transcription — were merged word by word, the rolling duplication of the
caption track removed, and obvious recognition noise corrected against the other
source and against plain sense. Where the two sources contradicted each other and
neither could be trusted, the passage is marked `[?]`. The result is not the
speaker's text and is not a verbatim rendering of either recognition; it is a
readable record of what was said, made faithful by using two witnesses instead of
one.

The caption track alone is unusable as a record: it carries the speaker's words
in near-unpunctuated fragments, mishears the speaker's own name throughout, and
mishears the mathematical vocabulary it exists to preserve. The Mistral
transcription is fluent and correctly punctuated, and it is confidently wrong in
places — it renders the speaker's name as "Dykstra" and "Edsker", his chair as
the "Slumber J. Centennial Chair", the urn as an "ear", and at 07:06 it turns
the preservation step into "B still holds" where the pattern requires `P`. Both
faults are recorded in the section they occur in.

## The recording is composite

The upload is titled as Dijkstra's lecture throughout, but its first minute is
not his. It opens with **Eric Schmidt**, vice president of the General Systems
Group of Sun Microsystems, inaugurating the University Video Communications
Academic Honor Series and introducing the first speaker: Dijkstra, at the
University of Texas at Austin, who "will present two exercises on reasoning about
programs". Dijkstra takes over at 01:31 and speaks for the remaining
fifty-three minutes. A record that attributes the introduction to Dijkstra
misstates the tape.

## The board at 07:17

The whiteboard is stable from 07:08 to 07:30 and carries the pattern the whole
first half of the talk is built on, written under the name C. A. R. Hoare:

```text
     {Q}   S   {R}

 {P}
   do  B →
      {P ∧ B}   S   {P}
   od {P ∧ ¬B}
```

The general triple `{Q} S {R}` is Hoare's; the rest is the repetition. `{P}` is
the side condition to be established before the `do`. `{P ∧ B} S {P}` is the
body: the same statement `S` with the invariant in its postcondition and the
invariant together with the guard in its precondition, which is what makes the
repetition a single induction rather than a chain of unrelated steps. `od` is
followed by `P` together with the negation of the guard.

Spoken, at 07:33: "P is called the invariant, B is called the guard, S is called
the statement, and the post condition is P and the negation of the guard." At
07:18, of the pattern as a whole: "here we have the general pattern that we will
use over and over again to prove things about the repetitive construct". At
06:12 he states the two premises — that `P` holds before the repetition, and
that the guard guarantees that upon completion of `S`, `P` still holds — and at
06:55 the consequence: `P` remains true no matter how often the statement is
executed, "and furthermore, because the repetition is terminated, we know that
the guard B is no longer true".

Termination is what supplies the negation of the guard, not the invariant: an
invariant that holds at every step of a repetition is consistent with a
repetition that never ends. That separation of partial correctness from
termination is announced at 02:34 and carried through both worked examples.

The last minute of the tape carries no speech in either source: the caption
track ends at 52:59 with a music cue and the Mistral transcription at 52:58. The
final 1:50 is unattended audio and nothing is recoverable there.

## Transcript
## 00:00–09:00

[00:00] [Music]

[00:39] The introducer, Eric Schmidt, vice president of the General Systems Group of Sun Microsystems, opens the recording: "Hello, I'm Eric Schmidt, vice president of the General Systems Group of Sun Microsystems. It's my pleasure to inaugurate the University Video Communications Academic Honor Series." In this series, he explains, renowned academics will present talks on topics of their choosing, for widespread distribution around the world to corporate, technical and academic audiences.

[01:02] "It's Sun's pleasure to sponsor the first speaker in this series, Dr Edsger W. Dijkstra, who occupies the [?] Centennial Chair of Computer Sciences at the University of Texas at Austin. He'll present two exercises on reasoning about programs. May I introduce Professor Dr Edsger W. Dijkstra." [Music]

Dijkstra begins, and it is he — not the introducer — who speaks for the remaining fifty-three minutes: "Welcome to this talk on reasoning about programs."

[01:38] "We all know that machines are so fast and stores are so big that they give us plenty of latitude to screw things up. And in order to prevent that, we have to prove that our programs indeed produce the results we would like them to produce."

[02:07] "This talk will have two themes. The one theme is the general structure of correctness arguments about programs. The underlying theme is how to keep the arguments as clean and as simple as possible."

[02:34] "One of the ways of achieving the latter is to separate our concerns as best as we can. One of the first separations of concerns we will encounter is the separation between total correctness and partial correctness — more precisely, between partial correctness and termination. Let me introduce the terms to you."

[03:02] "Total correctness is our final target. Total correctness means that the program will produce the right result. Partial correctness is a weaker concept: it only guarantees that the program will produce the right result if its execution terminates."

[03:35] "Now the introduction of the notion of partial correctness splits our concerns for total correctness into two separate ones. One of them is the concern for partial correctness and the other one is the concern for termination. We will see that the separation of these two concerns is really helpful, because the considerations about partial correctness are totally independent of proofs of termination."

[04:14] "In order to show you the general structure of those proofs, particularly for partial correctness, let me go for a moment to the blackboard." He will use a notation introduced by C. A. R. Hoare in the late sixties: "let S be a program fragment. Let Q be the initial condition and let R be a postcondition — precondition, postcondition, initial condition, final condition, these are alternative terms." Hoare introduced this to mean that if the initial state for the execution of the program fragment S satisfies condition Q, then, upon termination, the program will have taken the machine into a final state that satisfies condition R.

[05:35] "Now it is in this formalism that I will describe the general proof rule for a repetitive construct." The repetitive construct he takes is a very simple one, consisting of two things: a Boolean expression B, saying that the repeatable statement should be executed once more, then the repeatable statement, and finally the closing bracket, "od".

[06:12] "If, firstly, prior to the execution of the whole repeatable statement, a condition P holds — P, by the way, is called the invariant — and if S has the property that the initial validity of P and the guard guarantees that upon completion of S, P still holds, well, then… clearly S is such that its execution does not destroy the validity of P."

[06:55] "P holding at the beginning, P remains true no matter how often the statement is executed. So upon completion, P still holds. But furthermore, because the repetition is terminated, we know that the guard B is no longer true."

[07:19] "Now here we have the general pattern that we will use over and over again to prove things about the repetitive construct: do B, arrow, S, od." In the pattern, P is called the invariant, B is called the guard, S is called the statement, and the postcondition is P and the negation of the guard.

[07:50] "We shall now apply these patterns of reasoning to two different examples: the first one, in which the correctness concern plays the major role, and then a second one in which the termination concern plays the major role."

[08:09] "For the first example we shall consider a one-person game." The one-person game is played with a big urn full of pebbles, and each pebble is white or black — "we don't start with an empty urn". Besides that urn with pebbles, we have at our disposal as many white or black pebbles as the playing of the game might require. The game is such that a move is possible when there are at least two pebbles in the urn.

## 09:00–18:00

[09:01] Let us return to the game itself. To begin with, at least two pebbles must be in the urn, and as with all such one-person games the rule is that one goes on playing as long as moves are possible.

[09:21] For a move to be possible there have to be at least two pebbles in the urn, because the move is the following. What one does is: one shakes the urn, then looks in the opposite direction, puts one's hand into the urn and picks up two pebbles.

[09:42] One picks up two pebbles and looks at their colour, and then, depending on the colour of the two pebbles taken out, one puts a pebble into the urn. Now here are the colour rules, as stated.

[10:03] They are too complicated to remember, but we will return to them later. The idea is that if we take out two pebbles of different colour we put back the white one.

[10:20] However, if we take out two pebbles of equal colour we put a black one into the urn. And that is precisely the reason why we need to have pebbles in store, in stock, sorry: because you have taken out two white ones, then we have to put in a black one, so we have to have a sufficient supply of black pebbles, or quickly drying black paint, or something of that sort.

[10:53] Now this is the game. Not very exciting to play perhaps, but it is a worthwhile exercise to think about.

[11:07] The first question is: does this game terminate? Yes, it does, you see, because we start with the urn filled with a finite number of pebbles, and each move — taking out two pebbles and putting one back — reduces the number of pebbles by one.

[11:31] So as the game proceeds the number of pebbles in the urn decreases. Obviously that cannot go on forever, so this is an example in which the termination of the process is totally trivial to determine.

[12:09] However, the question we are going to address now is about the final state — the final state of the urn when we cannot continue the game any more. Now to begin with, and that is again a separation of concerns, we could try to analyse what we can say about this game if we totally ignore those complicated colour rules.

[12:53] If we take into account from the move only that in each move two pebbles are taken out and one is turned back, you see, the problem is that: given the initial contents of the urn, what can be said about the colour of the final pebble? Now this is cheating already a little bit, because the question says that there is a final pebble, one pebble left in the urn. Can we prove that?

[13:45] Yes we can, you see, because here is a description of the game being played, but totally ignoring the colours.

[13:59] So we introduce an integer variable, little k, and k is initialised with the number of pebbles in the urn, whatever their colour. Our first statement, little k becomes capital K, is the initialisation, and now we know that little k is at least one, because we knew that before the game was started the urn was non-empty. And here are the rules, here are the rest of the game, and there you get a repetition.

[14:39] And as long as there are at least two pebbles in the game, that is k at least two, we get the two steps of our move: k becomes k minus two, modelling that the two pebbles are taken out of the game, followed by k becomes k plus one, modelling that a pebble is put back.

[15:04] Now if you combine those two things, those two steps, the net effect of taking out two and putting one back is, of course, k becomes k minus one. And here you see the annotated program, with an annotation very much in the style as I showed on the blackboard: little k becomes capital K, and since capital K was at least one to start with, here we have the initial condition for the repetition, that little k is at least one.

[15:40] Then we get the repetition, do, our previous guard, k at least two, arrow, and at that moment we can assert the invariant P, that k is at least one; and also the guard B, k is at least two, so at that place we can assert k at least two.

[16:04] Now obviously the precondition k at least two guarantees that after the decrease, k becomes k minus one, k is at least one. So we see that the repeatable statement k becomes k minus one nicely maintains the truth of k at least one.

[16:28] So upon completion we know two things: that the guard is [?] — so k is no longer at least two, so k is less than two — and furthermore that k is at least one, which has only one solution, k equal one. So using the techniques of invariance we have proved the simple fact that our game terminates with one pebble in the urn.

[17:13] We are indeed entitled to talk about the final pebble. Let us now return to the original question, and that is: given the initial contents of the urn, what can we say about the colour of the final pebble?

[17:46] Well, this is clearly the moment to take the colours into account, so it is no longer sufficient to characterise

## 18:00–27:00

[18:00] So we characterise the urn by its total number of pebbles, and we do so by the two totals rather than by the one. Obviously we must know the number of black pebbles and the number of white pebbles, and what we used to call capital K, the initial number, we now call capital B plus capital W, for the initial numbers of blacks and whites. And there we are: little b and little w represent the number of black pebbles currently in the urn and the number of white pebbles currently in the urn. The first line of this little program is the initialisation, the initial filling of the urn.

[18:57] Now remember our move. Our move required at least two pebbles in the urn; you took them out and looked at their colours. Now they could be of different colours — there could be a white one and a black one. When is that a possible outcome? Well, when initially there was at least a white and at least a black one in the urn. So the condition under which we could pick up a mixed set, a white one and a black one, is that W is at least one and B is at least one. Well, that is the guard of the first alternative: you see, little W at least one and little B at least one, arrow.

[19:47] There I have sketched, as a reminder between braces, the rule of the game, and that says that when you take out a white one and a black one, a white one is returned, and then the assignment statement B becomes B minus one. We see the net effect of that move: the number of black pebbles is decreased by one, and the number of white pebbles is unchanged.

[20:20] The other two guarded commands deal with the cases in which two pebbles of equal colour are taken out. If B is at least two, two black pebbles can be taken out and a black will be returned again; the net effect on the filling of the urn is B becomes B minus one.

[20:50] The complicated case is when two white pebbles are taken out — well, that is the case where we might need the black paint, because W decreases by two and B increases by one. So here we have a description of the game that takes the changes in the filling — the colour, or the number of colours, in the urn — into account.

[21:28] Now a moment's inspection of these three possibilities tells us that, as far as change in the urn is concerned, we need not distinguish between the first two: they just decrease B by one and leave W as it is. The last one increases B by one and decreases W, but decreases it by two. So we see that whatever happens, W remains constant or W is decreased by two. In other words, W is only ever decreased by two, and that means that if W started even it will remain even, and if W started odd it will remain odd.

[22:36] And I show you the consequence of that on the blackboard. Our original invariant for the repetition was that K was at least one, but K is now equal to little B plus little W, so we translate that into: little B plus little W is at least one. And furthermore we have established that the parity of W does not change — that is, even little W is equivalent to even big W, capital W.

[23:22] And here we have answered our problem. You see, in the case of even W — we start with an even number of white pebbles in the urn, and it remains even — upon completion there is only one pebble in the urn, and that is odd, one is odd. So the result is that the final pebble is black. In formula, the final condition is that little W equals zero and little B equals one.

[24:19] Of course, if you start with an odd number of white ones in the urn, then the final pebble is white, because the final state is characterised by W equal to one and B equal to zero, these being the only solutions of B plus W being a natural number equal to one. So this settles the problem of what can be said about the colour of the final pebble.

[25:00] Now you may complain about this argument: the conjunct of the invariant, that the parity of W does not change, is an invention which in general you would be hard put to make. That complaint is to a certain extent justified. But fortunately the designing programmer lives in a different situation. He is not offered the ready-made program, the rules of the game that I have just described to you, and has to invent the invariant.

[25:44] In actual practice, when a programmer develops correctness proof and program hand in hand, he knows the invariant before the program has been written. And suppose that his task had been the following. You are given an urn with black and white pebbles, and you are requested to remove all but the last pebble from this urn. The way in which you may do that is: you are not allowed just to turn the urn over; you have to do it very carefully. You have to do that in a series of moves, and in each move you have to satisfy two constraints. The one constraint is that you have to take out two pebbles and put one back in.

## 27:00–36:00

…pebbles, and put one back in.

[27:08] Well, there are all sorts of games with such restrictions. There is the famous puzzle of the missionaries and the cannibals that have to cross a river, and there is a little boat, and the boat can contain two people, and one goes back to take the boat to the other side. So the rule two out, one in, is not too unusual.

[27:33] If you now impose upon someone the constraint that he has to do this in such a way that the parity of the number of white pebbles in the urn between moves remains constant, then precisely the rules of this game will come out. And of course the rules of the game — they were the analogue of the program.

[28:07] Well, that's what I wanted to tell about: the first example, where the termination is trivial, and the invariant and the partial correctness considerations take the majority of the load.

[28:33] [Music] Our next example is very different. In passing it shows another form of the interplay between programming and mathematics.

[28:58] In this example a mathematical theorem has to be proved, and the theorem is an existence theorem: it tells us that a certain result exists. We shall prove that theorem by translating it into a programming exercise, and writing a program of which we can demonstrate that it computes the desired result. Now, if you can construct a program that computes a result, that result certainly exists.

[29:48] Let me explain the problem first. We are considering capital N blue points in the Euclidean plane, and an equal number of red points in the Euclidean plane, such that no three of them are on the same straight line. That is clearly satisfied in this example.

[30:29] Furthermore, the next thing to do is that we consider a one-to-one correspondence between the blue points and the red points. That is, we form N pairs, and each pair consists of a blue and a red one: each blue point is coupled to a red point and vice versa, and the two points of a pair are connected by a straight line. There are a number of ways in which you can do so, and of course the number of them increases rapidly with the number of points. If there is one red and one blue point, you do not have any freedom at all: you can only pair them in one way. Here we have three of each, and then the number of ways, the number of one-to-one correspondences between the reds and the blues, the number of ways in which you can pair them, is N factorial. So in this case it is six.

[31:34] Now the theorem to be proved is that there exists a one-to-one correspondence such that none of the N line segments intersect.

[31:58] Here we have another pairing of the same six points, and here you see that there is still an intersecting pair. I think that my next slide shows a possible solution. My guess is that for this situation of red and blue points this is the only solution — in general the solution is not unique, but okay, here we have a solution: the N line segments connecting the points of the pairs do not intersect.

[32:56] Now we have to show that for any value of N this is true, that such a solution exists, and we'll do so by designing a program.

[33:17] Well, we are making ourselves now very, very easy. We say: well, our program operates on a single variable, a variable named Z, and it is of type one-to-one correspondence.

[33:36] Now — since there are, well, I should have said, it is of type one-to-one correspondence between the given N red points and N blue points, but that long sentence could not, didn't fit on the single view graph. So a variable Z, of type one-to-one correspondence, it just has N factorial possible values, each of them representing one of the N factorial one-to-one correspondences.

[34:17] Now, in any program with a repetition, it always starts with an initialization and then has the repetition. So the first statement is: initialise Z. And upon completion of that initialisation Z has a permissible value, that is, Z represents one of the N factorial one-to-one correspondences.

[34:44] Now, and here we have our little program. The program evaluates the Boolean Z has intersection, which is either true or false. If Z has no intersection, the repetition terminates and we are done.

[35:11] The only thing the program has to do is to change Z. Now, reading this program, it's quite clear that upon termination the value of Z is a one-to-one correspondence without intersections. So our only proof obligation is that —

[35:44] Well, as it stands we cannot prove that yet. You see, because our program as written is a little bit, a little bit non-deterministic.

## 36:00–45:00

[36:00] Non-deterministic, we have left completely open how that is initialized, and that is probably not very worrisome. What is worrisome — what an incompleteness — is more serious: we have not indicated how Z is going to be changed if it has an intersection. So let us investigate the situation that the statement change Z has to cope with.

[36:25] Now, listen, the preceding guard was "Z has an intersection", so the precondition Q of the operation change Z is "Z has one intersection, at least"; more intersections cannot be guaranteed. The as yet unrefined statement change Z therefore has to be designed in such a way that it copes with the situation of one intersection.

[37:10] Now, that is the situation we have drawn here: two red, two blue, two red-blue connections, and they intersect. The presence of this subfigure is the only thing we can rely on. How can the one-to-one correspondence be changed? Well, the only way we can do that is by changing the solid lines into the dotted lines. The red point in the left top was coupled to one of the blue ones, and now has to be paired to the other blue one.

[38:12] So the only freedom we have is to change those two pairings, those two connections — and the operation flip. I did not give it a name; I simply call it flip. I would like you to understand that in the design of that operation we do not have any choice, because the precondition Q only guarantees the existence of one intersecting pair of connections. Now, of course, we have to find the termination argument.

[39:26] Now, regrettably, we cannot argue that if we replace the solid lines by the dotted lines, the number of intersections decreases. Let me give you an example. We had the situation drawn that this blue and this red were connected, and this red with the other blue, and the suggestion was to re-couple this red with that blue, and that red with that blue. This intersection indeed has disappeared; however, we do not know how many other pairs were here, or, for instance, here — the fact that this new connection and this new connection may intersect.

[40:21] Heaven knows how many of the other intersections that we did not take into consideration. That tells us that, if we are looking for a termination argument, we cannot derive it from a decrease of the number of intersections; it may actually increase. But it tells us what we have to look for. We cannot count the number of intersections, because an intersection is the result of a pair of segments, and we do not know with how many unmentioned connections those two connections interfere.

[41:34] And it tells us that we have to find our termination argument from an argument that considers the connections on a more individual basis. Now, there is one great advantage in this example, and the advantage is that our state space has only a finite number of possible values. Our state space consists of the single variable Z, and the single variable Z has only n factorial different values. So this is a problem with a finite state space.

[42:29] And we can ensure termination if execution of the algorithm never revisits the same state again — if it never reaches a state it has already been in, the computation can never cycle. Now, this we can guarantee if we can define some sort of function on the state that decreases in each step. We are now no longer restricted to an integer function counting something, as we did in the previous example with the pebbles. So it is all right if we can find any real function defined on a one-to-one correspondence that decreases in this move.

[43:35] Now, the interference argument tells us that this function should be built up from contributions from the individual line segments. And now, if you look at it, it is quite clear what decreases, because there is such a thing as the triangle inequality: in each triangle, the sum of two sides exceeds the length of the third.

[44:10] Now, in this triangle, the sum of these two lengths is more than that one, and the sum of these two lengths exceeds the other, the dotted one. And we see that in this move the length of these — sorry, the sum of the lengths of these two connections — decreases. The other pairings in the one-to-one correspondence remaining what they are, we conclude that in the move the sum of the lengths of the line segments decreases.

[44:49] And that is sufficient to demonstrate termination, and hence our program terminates, and hence our program establishes a one-to-one correspondence that has no intersections.

## 45:00–54:49

[45:03] Two final remarks about this last programme. One might ask what has
happened to the invariant, capital P, of the original introduction. Well, that is
very simple. In this example capital P, the invariant of the repetition, is
identically true. You see, because the invariant states a condition that will be
satisfied all through the repetition, all states in which the invariant is false
are ruled out. But thanks to the introduction of our abstract variable of type
one-to-one correspondence — which we just postulated, and which can have n
factorial different values — the introduction of that abstract variable enabled
us to introduce a state space of n factorial states in which all states are
allowed. So nothing is ruled out, and P can be true, and hence can disappear from
the argument.

[46:26] In a more realistic computation, the one-to-one correspondence would
probably be represented as one of the permutations, one of the n factorial
permutations of the numbers from naught through n minus one, or something of that
sort, and you would probably use an array or a sequence for that. In that case
part of the invariant would be that that array or sequence represents one of the
permutations of the numbers from naught through n minus one. But by proper choice
of the abstract type one-to-one correspondence, we have eliminated, so to speak,
the invariant P from our considerations — so much in favour of abstract programs.

[47:40] Another remark to be made is to compare this argument with how classical
mathematics would formulate it. I think the classical mathematical argument would
be as follows. There is a finite number of one-to-one correspondences. Consider now
those one-to-one correspondences such that the sum of the lengths of the
connections is minimal, and suppose that one has an intersection. Now then comes
the same argument that we had, and you construct a shorter one-to-one
correspondence, and then you have a contradiction, and by a reductio ad absurdum
the theorem is proved.

[48:42] I think that our argument here is preferable for two reasons. First of
all, we have avoided the reductio ad absurdum and given a completely constructive
proof. Secondly, we are freed from the moral obligation that many a classical
mathematician feels, and that is: after he has given a proof as a sketch, he adds,
"Note that the shortest one-to-one correspondence need not be the only solution."

[49:24] In our case we do not need to do that, because the final value of our
programme can be any of the intersection-free one-to-one correspondences. If there
is another one-to-one correspondence, then, because we have left the first
statement, "initialise z", completely undetermined, it could initialise z with that
value and immediately the repetition terminates. So here we have a programme for
which each possible answer is possible, if I may say so.

[50:38] Now this concludes the treatment of the second example.

[51:00] I put it to you[?]: having seen how we can convince ourselves that
programs are indeed totally correct, please realise that if you have written a
programme and it is not correct, it is a little bit cowardly to say that your
programme had a bug. To call errors "bugs" is a very primitive, animistic
attitude: it suggests that the bug has a life of itself, and that you are not
totally responsible for it — that the mean little bug crept in behind your back at
the moment you were not looking. Well, this is not true. If the program is not
correct, you made an error.

[52:22] And my request, my prayer so to speak, is that you stop using the term
"bugs" for program errors, but call them what they are: errors — unless we change
our language and call an error an error. Programming and computing science have
not yet matured.

[52:57] Thank you for your attention.

[52:59] [Music]
