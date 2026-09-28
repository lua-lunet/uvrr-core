                                                SMAUG: Secure Mobile Authentication
                                                         Using Gestures

                                                           Christian A. Gorke and Frederik Armknecht

                                                                     University of Mannheim
                                                                       Mannheim, Germany
                                                        gorke@uni-mannheim.de, armknecht@uni-mannheim.de



                                               Abstract. We present SMAUG (Secure Mobile Authentication Using




arXiv:1708.04871v1 [cs.CR] 16 Aug 2017
                                               Gestures), a novel biometric assisted authentication algorithm for mobile
                                               devices that is solely based on data collected from multiple sensors that
                                               are usually installed on modern devices – touch screen, gyroscope and
                                               accelerometer. As opposed to existing approaches, our system supports
                                               a fully flexible user input such as free-form gestures, multi-touch, and
                                               arbitrary amount of strokes.
                                               Our experiments confirm that this approach provides a high level of ro-
                                               bustness and security. More precisely, in 77% of all our test cases over
                                               all gestures considered, a user has been correctly identified during the
                                               first authentication attempt and in 99% after the third attempt, while
                                               an attacker has been detected in 97% of all test cases. As an example,
                                               gestures that have a good balance between complexity and usability, e.g.,
                                               drawing a two parallel lines using two fingers at the same time, 100%
                                               success rate after three login attempts and 97% impostor detection rate
                                               were given. We stress that we consider the strongest possible attacker
                                               model: an attacker is not only allowed to monitor the legitimate user
                                               during the authentication process, but also receives additional informa-
                                               tion on the biometric properties, for example pressure, speed, rotation,
                                               and acceleration. We see this method as a significant step beyond exist-
                                               ing authentication methods that can be deployed directly to devices in
                                               use without the need of additional hardware.


                                         Keywords: Mobile Security, Mobile Authentication, Authentication Schemes


                                         1   Introduction
                                         Mobile devices such as mobile phones or tablets companion our everyday living
                                         and often store valuable personal data. Consequently, there is a strong need for
                                         secure and usable authentication schemes. The most popular approach is based
                                         on proving certain knowledge (“What I know”), i.e., the user has to insert a PIN
                                         or a pattern to gain access to the device. However, this approach suffers from
                                         the known difficulties of choosing and memorizing strong passwords. Moreover,
                                         it is subject to shoulder-surfing or smudge-attacks [2,6]. While this is true for any
                                         type of devices, these attacks are particularly dangerous for mobile devices where


                                                                                  1
users have to frequently log-in while being outside of any secure environments
like home or offices. Even worse, often the device gives some visual feedback
on the entered password, which makes these attacks even easier. Consequently,
alternative authentication mechanisms that are not based on knowledge (only)
need to be considered.
    A straightforward approach would be to include a second device (“What I
possess”), e.g., some smart-card, but this negatively impacts usability. A further
alternative is to use biometric properties (“What I am”). Apart from the fact
that these methods often have their own security problems [14,40,15], a further
drawback is that this requires additional hardware, e.g., an iris scanner. But any
additional piece of hardware increases the size, the power consumption, and the
price of the device, directly contradicting the market requirements.

1.1   Contribution
In this paper we present a novel biometric assisted authentication algorithm for
mobile devices dubbed SMAUG (Secure Mobile Authentication Using Gestures)
that overcomes all these limitations mentioned earlier. The idea is based on
the observation that modern mobile devices are equipped with a multitude of
different types of sensors like the motion sensors gyroscope and accelerometer.
That is, users exhibit characteristic attributes like typing rhythm, gait, voice,
and movement in general that can be measured with built-in sensors of a device
and processed by an appropriate authentication algorithm. For authentication
based on SMAUG, a user starts with registering a gesture (enrollment phase)
and authenticates to the device later on by re-drawing the registered gesture
(verification phase). Here, a gesture is any interaction with the device regarding
the touch screen or motion sensors, e.g. a drawn shape on the surface of the
device, moving the device through space, or both simultaneously. Note that
during touching the device, its position and rotation is changed depending on the
position where it is being touched by the user. During both the enrollment and
verification phase, a number of sensor data are collected. In order to authenticate,
our algorithm combines these information to enable multi-factor authentication:
a user needs to know what gesture has to be made but also how the gesture
needs to be performed. Hence, the user re-draws the earlier registered gesture
and, if done by the legitimate user, he will be validated correctly with a high
probability. On the other hand, an impostor will be rejected by SMAUG with
a high probability. Please note that SMAUG supports multi-gesture login, that
is the user may register more than only one gesture. At the beginning of the
verification phase SMAUG randomly chooses a gesture out of this set of all
registered gestures, but possibly hiding the name of the gesture. Our scheme
exhibits a number of remarkable features as follows:

No Dedicated Hardware Required: Our algorithm uses only hardware that
   is commonly found in mobile devices: a touch screen, an accelerometer, and a
   gyroscope. This saves costs and makes SMAUG accessible on a huge number
   of devices.


                                         2
Fully Flexible Input: As opposed to previous approaches, the user can choose
    any arbitrary gesture: it does not need to be chosen from a predefined set,
    respects multi-touch, and can consist of as many strokes as desired at the
    same time. Furthermore, any number of gestures may be registered. In other
    words, SMAUG does not limit form and length of the user’s gesture.
Efficient Feature Extraction and Detection: We explain in detail how col-
    lected data can be combined in order to extract over 320 features for each
    gesture. Utilizing dynamic time warping allows us having efficient feature
    extraction and detection. Furthermore, we introduce individualized weight-
    ing for each user, gesture, and even feature for a very adaptive and distinct
    verification process. SMAUG employs not only the features of single sen-
    sors, but fuses them in different ways. The motion sensors are synchronized
    and interpreted as a single sensor, without excluding individual sensor fea-
    tures. Data from the touch screen is structured in a way which allows for the
    extraction of stroke dependent features for each single stroke in addition to
    features applying to the whole gesture at once. Merging everything together
    yields an entangled net of features allowing to distinguish between users.
    Finally, by supporting seven system parameters and four security parame-
    ters we provide a high level of flexibility for adapting the behavior of the
    algorithm, depending on the system and policies, e.g., allowed retries after
    failed login attempts or sensitivity of the verification. In addition, we specify
    default values to allow a start out of the box.
High Security and Usability: We implemented a prototype of SMAUG on
    Android 5.1.1 and tested it on a Nexus 5 device. Note that SMAUG is inde-
    pendent of operating system and device, as long as it supports touch screen,
    gyroscope, and accelerometer. The results show that involving the efficient
    feature extraction and individual feature weighting pays off. SMAUG is ca-
    pable to tell apart registered users from attackers with very high probability.
    We stress that we consider the strongest possible attacker model: an attacker
    was not only allowed to watch the legitimate user drawing the identifying
    gesture each time, but also receives additional information on the biomet-
    ric properties, for example pressure, speed, rotation and acceleration. We
    are not aware of any other authentication scheme based on this type of data
    that achieves a similar level of accuracy and security. Surprisingly this strong
    level of security is already achieved even if the user has chosen a very simply
    figure like the letter “A” (denoted by gesture A).
    With respect to usability, the registration process remains modest. A user
    is required to perform a gesture ten times to provide sufficient data to the
    algorithm for gesture registration. This is one of the system parameters men-
    tioned above and may be changed. However, we found that ten is well chosen
    between time time investment and security. Being the most time-consuming
    part, however, registration process needs to be done only once. After the
    features are extracted and processed, the authentication process itself takes
    only a fraction of a second. Also note that we do not employ continuous
    authentication, therefore our algorithm works offline and the privacy of the
    user is untouched as well as the battery life.


                                         3
We also want to point out that biometric schemes which rely on direct biometric
properties, such as iris scan or fingerprints, have a real-world application problem
when it comes to leakage of this data. In other words, if such a biometric property
is leaked, it is not safe anymore, i.e., (partially) public known. However, a user
can change these only a very limited time, in the case of fingerprints only nine
times. Hence, we think a scheme like SMAUG which is based on the features
of biometric properties, and not the properties itself, is much more secure in
the long term. Removing old gestures and adding new ones is a matter of a
few seconds and can be repeated arbitrarily often. There is no limit besides the
hardware storage and the user’s imagination regarding the amount of gestures.
This feature is known from traditional authentication methods like passwords
or certificates.
    With respect to usability, a user may want to use SMAUG with the same
gestures on each of her devices. However, since SMAUG does not exchange
information from any device to another, hence a user has to enroll the same
gesture on each device. If SMAUG is not only used for device authentication
but for service authentication on multiple devices, the same gesture may still
be used. This can be realized by storing an additional device identifier at the
authentication service, which holds the user identification and gesture templates.

1.2   Comparison to State of the Art
Compared to existing work, our scheme SMAUG provides the richest set of ca-
pabilities: multi-gesture (arbitrary amount of gestures), free-form gesture (ges-
tures are not restricted in form or length), multi-touch (arbitrary amount of
fingers used at the same time), multi-stroke (each finger can draw an arbitrary
amount independent of other fingers), multi-factor (combining biometric data
with knowledge), multi-sensor (touch sensor, gyroscope, accelerometer), sensor
fusion (combined evaluation of sensor data, especially gyroscope and accelerom-
eter data), and graphical password support (background images for gestures)
at the same time. Additionally, SMAUG does not use continuous authentica-
tion and hence works offline, i.e., it runs on the device itself without the need
to communicate to another party. Furthermore, SMAUG is independent of the
operating system and hence can be implemented from most popular to rare
systems. Moreover, SMAUG provides security with respect to the strongest at-
tacker model considered so far, i.e., an attacker who has full knowledge on how
the gesture looks like and how it has been drawn.
    In Table 6 on page 43 we give an overview and comparison of mobile authen-
tication schemes as well as their capabilities. Each row in this table refers to the
work given in the first column. A (red) “no” states a missing feature, while a
(green) “yes” denotes that the feature is fully supported in the according work.
Other entries, like (yellow) “partly” or (yellow) “yes” denote a general support
of a certain feature, but it is a restriction in comparison to other work, e.g.,
if only one operating system is supported. Note that the colors of “yes” and
“no” are interchanged for continuous authentication since it relates to loss of
privacy, which we interpret as a drawback of such schemes. We are not aware


                                         4
of schemes which run this operation efficiently on the device itself. As the table
shows, our algorithm SMAUG is rich on features and steps ahead of existing
work, providing the richest set of capabilities.

1.3   Outline
This paper is structured as follows. In Section 2, we describe the basic compo-
nents and the considered attacker model. Section 3 provides an overview of the
SMAUG scheme which is divided into an enrollment phase and a verification
phase. These are explained in detail in Section 4 and Section 5, respectively.
Section ?? presents an implementation of our scheme and discusses various ex-
perimental results. Section 7 gives an overview of related work and Section 8
concludes the paper including open questions for future research.


2     Preliminaries
In this section we explain the basic components of our authentication algorithm
SMAUG and the considered attacker model.

2.1   Sensors
SMAUG utilizes three types of sensors that are commonly found in mobile de-
vices: touch screen, gyroscope, and accelerometer. Touch screens report so-called
touch events, each consisting of tap coordinates, pressure, size of the pointer, and
a touch event action. A pointer is a general term for input devices used on the
touch screen – usually this is a finger or a pen-like device. Modern mobile devices
also support multi-touch events, i.e. allowing more than one pointer at the same
time on the touch screen. When multiple pointers are used, each change of one
pointer results in a new touch event of all active pointers. We call this set of
events a touch event set. If only one finger at the same time is being used on the
touch screen overall, we denote this as single-touch. Hence, all other cases are
called multi-touch. Besides the touch sensor, the gyroscope reports the change of
the angular rate for all three space dimensions while an accelerometer measures
the acceleration in each space dimension.
    In addition to the touch screen, we make use of two motion sensors, usually
implemented in hardware as MEMS (microelectromechanical systems) sensors
due to their size and energy consumption. A gyroscope reports the change of the
angular rate for all three space dimensions while an accelerometer measures the
acceleration in each space dimension. Every modern operating system supporting
accelerometers also provides a linear acceleration value which removes earth’s
gravitation in the measurement. In this paper, when we write accelerometer,
we always refer to the linear one. Important for our authentication algorithm is
the frequency of these sensors which determines the accuracy of the measured
values. While they work with very high precision, the frequency is usually limited
by the operating system to avoid an inflow of too many data from the sensors.


                                         5
Thus, we will operate on the highest frequency that is allowed by the operating
system, but at most 200Hz. For the touch screen, this is usually 30Hz or 60Hz,
which is sufficient for SMAUG. Each measurement of the motion sensors is
called a motion event. SMAUG considers not only the measurements of each
motion sensor, but also combines the motion sensor values into a new set, what
we denote by motion fusion. This is explained in detail in Section 4.3.6.

2.2   Gestures
We denote a simultaneous sequence of touch and motion events as a gesture.
Each gesture in SMAUG consists of an arbitrary number of strokes. A stroke is
a sequence of touch events belonging to the same pointer. This sequence starts
with a down event (touching the screen) and ends eventually with an up event
(releasing the pointer from the screen). For our authentication algorithm we
require at least one stroke to be part of a gesture. A stroke with drawing length
zero is called a point. In comparison to strokes, a point has a time length but no
distance. For our authentication algorithm we also allow multi-strokes, meaning
a gesture can consist of an unlimited amount of simultaneous strokes. Observe
that this may take place at overlapping time periods, that is one stroke may
end and a new one starts while another is still active. Furthermore, between two
strokes it is not required to stay in contact with the touch screen. We call a stroke
gap the time period between strokes where no touch event happens. However,
note that within stroke gaps, motion data events still occur. A stroke run is a
set of strokes separated by stroke gaps. Finally, we define a closed gesture to be
a gesture without stroke gaps. SMAUG supports all theses features to allow for
very individual inputs. This covers a wide range of gestures, from drawings and
signatures, to letters, rhythms, and even movements of the device in space due
to the combination of touch and motion sensors.

2.3   Dynamic Time Warping
One core aspect of authentication algorithms based on gestures is that a gesture
of a user needs to be mapped to already stored data such that later on, this
gesture will be recognized if coming from the same user. Here, one has to cope
with two contradicting requirements, security and robustness, which excludes a
number of existing techniques as we elaborate now.
    Due to the security requirements, it is important that the order of (overlay-
ing) strokes is registered. Thus, image recognition algorithms cannot be used
here since strokes may overlap, e.g. due to multi-stroke and multi-touch. This
holds in particular for algorithms that verify handwritten signatures.
    An alternative seems to be machine learning algorithms. However, the class
of supervised algorithms learns a classification rule by getting valid and invalid
examples, e.g, Support Vector Machines or k-nearest neighbor. In this case, the
valid examples would be the gestures executed by the legitimate user. However,
it remains unclear how and which invalid examples should be generated. For
example, if they stem from different persons, they will significantly different from


                                         6
the target gesture with high probability which negatively impacts the quality of
the learning result. A way out seems to be to limit the possible gestures to a
given set of gestures which have to be used by every user. Then the gestures
can be compared to each other. However, this would contradict our goal that we
want to offer the maximum flexibility (and entropy) when choosing the gesture.
On the other hand, unsupervised algorithms will either be too generous, that
is very robust but not secure, or need too much computing power which is not
provided locally on mobile devices.
   An even more natural choice would be to use one of the already existing
gesture recognition algorithms, for example $1 and $N which cover single-touch
multi-stroke cases [38,1]. However, they ignore the order of the strokes, may add
additional strokes to reduce multiple strokes back to a single one, and are in
general too “forgiving” for bad inputs, e.g., by resizing and rotating inputs. This
again increases robustness drastically, but decreases the level of security.
    Given these considerations, we opted for the dynamic time warping algorithm
(DTW), originating from speech recognition [29]. DTW can be used to measure
the distance between two sets, or the similarity between two sequences for real
values – being 0 for equal sets. DTW comes with the property that the sequences
don’t need to have the same number of elements and don’t have to be normalized.
For example, the values of the x-coordinates of one stroke can be compared
directly to the x-values of another stroke. The result is a measurement of their
distance, being 0 if both sets are equal. The data extracted by SMAUG of a
gesture contains plenty of these temporal sequences and is therefore well suited
for DTW. Plus, this technique was already used in earlier work to recognize
biometrical inputs [19,36].
    We will now briefly define the DTW algorithm. Let N := {N1 , . . . , Nn } ∈ Rn ,
M := {M1 , . . . , Mm } ∈ Rm . We set δ(a, b) := |a − b| as the distance function
for (a, b) ∈ (N, M ). The dynamic time warping algorithm DTW(N, M ) is then
defined as given in Algorithm 1, where [n] := {0, . . . , n}.



 Algorithm 1: Dynamic Time Warping (DTW).
   Input: N = (N1 , . . . , Nn ) ∈ Rn , M = (M1 , . . . , Mm ) ∈ Rm
   Output: DTW(N, M ) := δ(n, m)
 1 δ(i, j) ← 0 ∀ i ∈ [n], j ∈ [m]
 2 for i ← 1 to n do
 3      δ(i, 0) ← ∞
 4   for j ← 1 to m do
 5       δ(0, j) ← ∞
 6 for i ← 1 to n do
 7     for j ← 1 to m do
 8         δ(i, j) ← |Ni − Mj | + min (δ(i − 1, j), δ(i, j − 1), δ(i − 1, j − 1))

 9   return δ(n, m)



                                             7
2.4    Adversary Model

Our goal is to provide protection against the strongest possible attacker model.
Therefore, we consider an adversary who is allowed to observe the whole enroll-
ment phase. This includes direct vision to the touch screen of the device and
additionally a brief explanation on how the input has been done. Clearly, this is
stronger than shoulder-surfing or a smudge attack, since the adversary gets all
of this information for free. One remarkable property of SMAUG is that indeed
security against this type of attacker is given.


3     Algorithm Overview

In this section we give an overview of SMAUG, our proposed biometric authen-
tication algorithm for mobile devices. This includes notation, algorithm work
flow, description of the algorithm modes, and an overview of all system and
security parameters.


3.1    Notation

In a nutshell, the algorithm operates on two categories of data: meta data and
sensor data. The data belongs to a certain gesture G, we will employ G to refer
to an arbitrary gesture. The meta data set contains general information and will
be denoted by M. The sensor data set comprises three types of measurements:
touch data T (0) , accelerometer data A(0) , and gyroscope data G (0) . Furthermore,
fusion data F (0) will be computed from both motion sensor data sets. The data
sets will be processed and altered by different algorithms. New results of com-
putation on these data sets will be denoted by increasing superscript indexes,
i.e., T (0) , T (1) , T (2) , . . . and so on.
     The sensor data sets have a similar structure. Each set is represented by
a matrix over real values where a row represents one data event during the
measurements and each column represents one property of the data. For example,
each row of T (0) represents one touch event which by itself is a vector where each
entry, i.e., column, represents one property, for example the pointer’s pressure.
                                                                                    (k)
When we refer to a specific event, i.e., row, we use the notation Ti to address
                                       (k)
the i-th touch event of T . Moreover, if x refers to a certain property, we
denote by T (k) [x] the column that stands for property x To refer to the value of
                                                              (k)
a property x within the i-th event, we write Ti [x]. We use analogous notation
with respect to the accelerometer data A(k) and gyroscope data G (k) . To keep
                                             (k)
the description short, we use             S     to refer to the data sets G (k) , A(k) , and F (k)
                                     (k)
at the same time. By T                    r
                                            we refer to the data set of round r, we omit this
subscript if r is clear from the context, for example inside an algorithm. Here,
a round describes how often a user has to enter the same gesture during the
enrollment phase in order to correctly register it.
     By U we denote the user, who is the owner of the device and the input data.
An unknown user and possible impostor is denoted by U∗ .


                                                8
3.2     Algorithm Work Flow

In a nutshell, SMAUG runs on mobile devices and consists of an enrollment
phase for each gesture a user wants to use for authentication. SMAUG extracts
specific features of this data which represent the knowledge and behavior of
the user during login. This data is personalized through certain feature weights
which depend on the form of the gesture and the user at the same time. It is very
important to note that the selected features of the data as well as the architecture
of SMAUG allow for a combination of arbitrarily inputs, such as free-form,
multi-touch, multi-strokes, arbitrary number and length of strokes, resulting in
a flexible input system only limited by the user herself. As a second part, the
verification phase asks the user to enter a randomly chosen gesture out of all
gestures entered during the enrollment phase and records a single gesture input.
Then, the same certain data features are extracted as before and compared to
the previously stored authentication gesture. If the verification or authentication
input consists of the same data features as the corresponding one from the
enrollment phase, the user is authenticated. However, due to the personalized
weights and feature handling, an impostor is not able to authenticate with high
probability.
    As already mentioned, our proposed scheme consists of two different phases:
an enrollment phase where the user U registers a gesture G, and a verification
phase where new input is compared to the registered gesture G. An overview
of the algorithm work flow is depicted in Fig. 1. This schematic shows the en-
rollment phase and verification phase of SMAUG, where the rounded boxes
(red) are performed by the user, and the rectangular boxes (green) are done by
SMAUG. At least one enrollment phase (gesture registration) must be executed
before a verification phase (authentication) can take place. The output is either
an authentication failure or success which is output to the user, e.g. by device
unlocking, online service authentication, and so on. At the end of this section
we also give a brief note on the real-world instantiation of SMAUG.


3.2.1    Enrollment Phase
First, in the data processing stage, the user inputs her gesture G overall PE times,
yielding the data sets (M, T (0) , G (0) , A(0) )r for each round r = 1, . . . , PE . Then,
each data set gets processed and features of the data, such as coordinates and
other properties, are extracted for each round r. Finally, the templates for this
gesture are generated and stored in a template database, containing all relevant
information for each gesture.
    Second, in the weight computation stage, we do a cross validation of the user
data. That is, for each round r the appropriate data set from the data processing
stage is being treated as a single test input and compared to the already stored
templates of the gesture. We then count the number of erroneous deviations for
each feature and compute the weight of each feature depending on the num-
ber of errors for this specific feature. The weights represent the “stability” or
“habitages” of the user mapped onto the feature set. For example, if a feature


                                            9
         Enrollment Phase                                  Verification Phase



               Choose Gesture              Input Prompt




   repeat
                Input Gesture              Input Gesture
  PE times
                                                                       rep
                 Fetch   Data               Fetch   Data                   eat

                                                                            at m
                Postprocessing             Postprocessing                            P
                                                                                 ost
                                                                                   V
               Process   Data             Process   Data
                                                                                       b er
                                                                                    num
              Feature Extraction         Feature Extraction
                                                                                         of t
                                                                                              s
                         Template                                                         ime
              Generate
                                         Compute    Pre-Template
             Template Database

              Compute    Weights             Validation
                                                                                         Authentication
              Weights Database                                                              Failure
                                            Fetch   Result         7
              Compute    Thresholds

                                                                                         Authentication
                 Thresholds                 Verification
                                                                                            Success
                                                                       3



Fig. 1. Schematic flow chart of the enrollment phase and verification phase of SMAUG.
The steps displayed as rounded boxes (red) are done by involvement of the user, the
rectangles (green) part by SMAUG. This respectively holds for the arrows. The value
PE is a system parameter and can be changed, default value is 10. Also, the value of
PV is free of choice and by default 2. At the end, verification outputs a binary decision
(to the user): authentication success (3) or authentication failure (7).




failed in nearly all PE tests, it will have a very low weight. But if a feature
in all tests always succeeds, it will have the maximum weight, therefore being
very important for this gesture and user. Here, feature failure relates to high
deviation from all other data sets regarding this feature compared to a specific
data set. In the same way, success refers to a low deviation. Furthermore, each
feature belongs to one of three classes, called tiers of features. The weight also
depends on the value of this tier, reflecting the influence of certain features. For
example, if the x and y coordinates of the touch sensor are horribly wrong, but
the Kurtosis of one axis of a motion sensor is perfect, the first must weight in
more. However, the second is important for our strong attacker model. The set


                                           10
of all weights of the gesture and its strokes is denoted by W. Finally, out of W
two thresholds Θ1 , Θ2 get computed.
    The inputs and outputs of these subsidiary algorithms (or stages) as well as
their order is shown in the algorithmic description of Algorithm 2. Strictly speak-
ing, the for loop in Line 1 does not necessarily mean that all three algorithms
have to be performed, data gathering may be done completely separated from
the rest. The enrollment phase of SMAUG is explained in detail in Section 4.



 Algorithm 2: Enrollment phase of SMAUG.
   Input: User U, gesture G
   Output: M, T , S, W, Θ1 , Θ2
 1 for r ← 1 to PE do
 2     (M, T (0) , G (0) , A(0) )r ← DataGathering(U, G, r)
 3     (T (1) , S (1) )r ← PostProcessing((T (0) , G (0) , A(0) )r )
 4     (T (1) , T (2) , T (3) , S (2) )r ← FeatureExtraction((T (1) , S (1) )r )
 5 (T , S) ← TemplateGeneration(M, {(T (1) , T (2) , T (3) , F (1) , S (2) )r }1≤r≤PE )
 6 W ← ComputeWeights(T , S)
 7 (Θ1 , Θ2 ) ← Thresholds(W)
 8 return M, T , S, W, Θ1 , Θ2




3.2.2    Verification Phase

The verification phase is executed each time an authentication attempt takes
place. This phase is divided into two stages which are repeated until a successful
input was made, but at most PV times.
    First, in the data is gathered processed, that is an unknown user U∗ enters
a single data set (Mf, Te (0) , Ge(0) , Ae(0) ) by performing a gesture G* . This data set
is being processed and features are extracted.
    Second, to perform the verification, the data set and features from the pre-
vious stage are compared to the data set and features belonging to the original
gesture template (T , S) which are kept in the template database. We count the
number of new errors for each feature of this comparison as If          F . Here, an error
is defined as too much deviation from the stored gesture template. We also in-
clude all weights of the errors from W, yielding the sum I          gW over all weighted
errors. If IW ≤ Θ1 and IF ≤ Θ2 , the algorithm returns the decision true and
           g                f
the authentication is successful, else false.
    Algorithm 3 gives an algorithmic description of this phase. The verification
algorithm computes a binary decision dec which denotes if the authentication
attempt was successful. We explain the verification phase of SMAUG in detail
in Section 5.


                                                  11
 Algorithm 3: Verification phase of SMAUG.
   Input: T , S, W, Θ1 , Θ2 , U∗ , G*
   Output: Decision dec
 1 dec ← false
 2 t←0
 3 while t ≤ PV do
 4     (Mf, Te (0) , Ge(0) , Ae(0) ) ← DataGathering(U∗ , G* , t)
 5     (Te (1) , Se(1) ) ← PostProcessing(Te (0) , Ge(0) , Ae(0) )
 6     (Te (2) , Se(2) ) ← VerificationFeatureExtraction(Te (1) , Se(1) )
 7     (I
        g W , IF ) ← Validation((T
               f                         e (2) , Se(2) ), (T , S), W)
 8     if Verification(I     g W , IF , Θ1 , Θ2 ) = true then
                                    f
 9          dec ← true
10          t ← PV
11       t←t+1
12    return dec




3.3     Algorithm Modes


SMAUG supports three different modes which can be combined in any way.


Background Image Mode. This mode is enabled whenever the owner of the
   gesture chooses to add a background image to the gesture during the enroll-
   ment phase. This picture may give helping indicators for how and especially
   where to perform the gesture on the touch screen. As shown in the results
   of our experiments, cf. Section 6.3, this does not impact the security of
   SMAUG since we are using a different set of security parameters for this
   mode.
Multi-Touch Mode. The algorithm computes the number of maximum point-
   ers at the same time for each gesture during the feature extraction phase, cf.
   Section 4.3.2. Therefore this mode is chosen automatically dependent on the
   gesture data. For this mode we also use a different set of security parameters,
   which are explained in Section 4.6.
Secret Gesture Mode If this mode is disabled, during the login on the device
   a message will be displayed instructing the user which gesture to enter by
   displaying the name of the gesture. If this mode is enabled, this message is
   hidden. By default, this mode is enabled, hence SMAUG performs multi-
   factor authentication. If the user has created multiple gestures and flagged
   as secret mode, she must assure that each of them has different backgrounds
   to distinguish between them. We do not employ specific security parameters
   for this mode, since our adversary model covers a well-informed attacker, no
   matter of this mode.


                                                12
3.4   System Parameters
We summarize all seven system parameters and the secret gesture mode in Ta-
ble 1. For each parameter, a default value is given as well as a brief description.
The values of the security parameters are given in Section ??, Table 3.


  System Parameter Default Value Description
           PE                  10        Number of times a user has to repeat a
                                         gesture during enrollment phase, cf. Sec-
                                         tion 4).
           PV                  2         Maximum number of additional authenti-
                                         cation attempts during verification phase,
                                         cf. Section 5.
           PT 1               0.75       Tier one feature weight multiplicator, cf.
                                         Section 4.5.2.
           PT 2                1         Tier two feature weight multiplicator, cf.
                                         Section 4.5.2.
           PT 3                2         Tier three feature weight multiplicator, cf.
                                         Section 4.5.2.
          POS                 150        Time offset in milliseconds for motion data
                                         before first touch event, cf. Section 4.2.3.
          POE                 100        Time offset in milliseconds for motion data
                                         after last touch event, cf. Section 4.2.3.
  Secret Gesture Mode       Enabled      Display gesture name during verification
                                         phase, cf. Section 3.3.

   Table 1. Default values and description of all system and security parameters.




3.5   Notes on Real-World Instantiation of SMAUG
The description of both the enrollment phase and verification phase above,
shows how SMAUG works for a single gesture. However, an instantiation of
SMAUG in practice will work slightly different. As mentioned in the introduc-
tion, SMAUG supports multiple gestures at the same time. That is, a user can
have as many gestures as she wants, as long as they all have different identi-
fiers, i.e., descriptive labels such that the system can distinguish them like “A”.
This is stored in the metadata employed in the enrollment phase. Therefore, the
enrollment phase of SMAUG takes as input the user U and a set of gestures
G1 , G2 , . . ., which can be increased at any time. Then each single gesture registra-
tion is performed as described in Algorithm 2. Note that the algorithm modes
are also gesture dependent.
     For the verification phase, a real-world-instantiation of SMAUG will ran-
domly choose one of all registered gestures of the user. This is part of the input
prompt, cf. Figure 1. Furthermore, if the authentication fails more than PV + 1


                                          13
times, a fallback mechanisms will be called. This mechanism is out of scope for
this paper, but may be implemented as a password authentication, or increasing
the time between authentication trials.
    In the remainder of this paper we will describe the algorithms for one ges-
ture, if not stated otherwise. An extension merely adds another index to most
variables.


4     Enrollment Phase
The enrollment phase is repeated PE times for a gesture G and each repetition is
called a round r. According to our experiments, choosing PE = 10 achieves the
highest amount of security while still providing a reasonable level of usability.
Before the first round, the user chooses the name and mode of the gesture. At the
end of this phase, the device stores a kind of fingerprint of the gesture defined
and personalized by the user who entered the gesture in form of a template
and according weights. Note that the user can create as many gestures as she
wants to. However, for the sake of readability we focus on one gesture within
this section.

4.1   Data Gathering
The first step is having the user U choosing the mode and entering the gesture G
for each round r. The algorithm DataGathering records the following data sets
during this phase:
                                         
                   M, T (0) , G (0) , A(0) ← DataGathering(U, G, r).
                                     r

Meta Data M: This includes the name of the gesture, a unique gesture iden-
   tifier, the background image, the modes, and the round r.
Touch Data T (0) : When the user touches the touch screen, touch events get
   recorded and stored in T (0) . The user is allowed to have stroke gaps in
   his gesture. A single touch data event is a vector and contains a unique
   gesture identifier, the round number r, and the following properties: sensor
   type (touch screen), event time (timestamp ν ∈ N0 in nanoseconds), pointer
   ID (identifier for a pointer, starting at 0; for a stroke the pointer identifier
   always stays the same), pointer number (counter starting at 0 and counting
   up until the number of pointers in a single touch event set), touch action
   (action of the touch event), x- and y-coordinate x, y ∈ N0 of the pointer
   on the touch screen, pressure P ∈ [0, 2] (normalized pressure of the pointer
   on the touch screen; note that this is called pressure but measures also the
   size of a pointer on the touch screen), size S ∈ [0, 1] (normalized size of the
   pointer on the touch screen).
   These values represent the maximum amount of physical information the
   device gets from a pointer on currently available touch screens. The coordi-
   nates describe the layout of the gesture directly, while the pressure, size and


                                         14
  system time values represent strong biometric identifier. Recording is done
  with the highest frequency possible, for example 30Hz.
Motion Data A(0) , G (0) : The recording of motion data, i.e., gyroscope and
  accelerometer data, starts immediately after the user begins with each round
  and ends on finishing each round. The user notifies the system when a round
  has finished by pressing a button on the screen. Recording is done with the
  highest frequency possible, for example 200Hz.
  As mentioned before, a motion data record (A(0) and G (0) ) is a matrix where
  each row consists of the values of a single motion data event. Each such event
  contains a unique gesture identifier, the round number r, and the following
  properties: sensor type (gyroscope or accelerometer), event time (timestamp
  ν ∈ N0 in nanoseconds), three sensor values reported directly from the sensor.
  While these raw values are already quite characteristic for a user we will
  describe the characteristic features below which can be computed from the
  sensor values.


4.2     Post Processing

The post processing algorithm corrects and selects the significant gesture data
from the data gathered in Section 4.1 for each round r. It also computes the
motion fusion data. We denote this by

          (T (1) , G (1) , A(1) , F (1) )r ← PostProcessing((T (0) , G (0) , A(0) )r )

and explain the sub-algorithms of the post processing in the following. Remember
that S (1) = (G (1) , A(1) , F (1) ).


4.2.1    Touch Event Action Correction
The purpose of this first algorithm is to correct touch events in the data set which
have an incorrect touch event action for our purpose. These incorrect measure-
ments appear due to the operating system’s reporting of multiple touch events
happen at the same time, i.e., releasing a finger from the screen while moving an-
other one. This is needed to identify strokes of the gesture later on. In short, this
procedure scans for all pointers that are still active after the TOUCH POINTER UP
event and corrects these to a TOUCH MOVE event. The TOUCH POINTER UP event is
triggered when more than one pointer is active on the touch screen and one of
them is being released. The event TOUCH MOVE denotes motion of a single pointer.
We refer to Table 2 for an example of this process. All changes are performed
and stored in T (0) . Remember that pointers are a general term for input object,
such as finger or pen, see Section 2.1.


4.2.2    Strokes Determination
Observe that neither T (0) nor S (0) contains information about strokes. Due to
the previous correction, we now are able to detect and distinguish strokes. Since


                                              15
    Event Time Event Set Pointer ID Pointer Number Touch Action
         ...           ...          ...              ...               ...
         208            5            0                0         TOUCH POINTER UP
         209            5            1                1         TOUCH POINTER UP
         210            5            2                2         TOUCH POINTER UP
         211            6            1                0            TOUCH MOVE
         212            6            2                1            TOUCH MOVE
         ...           ...          ...              ...               ...
                                    ⇓ correction ⇓
         ...           ...          ...              ...               ...
         208            5            0                0         TOUCH POINTER UP
         209            5            1                1            TOUCH MOVE
         210            5            2                2            TOUCH MOVE
         211            6            1                0            TOUCH MOVE
         212            6            2                1            TOUCH MOVE
         ...           ...          ...              ...               ...

Table 2. Example for TOUCH POINTER UP correction for two event sets in Android.
Without the correction, we would not be able to recognize which pointer left the set
of active pointers on the touch screen. The problem also exists for pointer joining, but
this can be fixed during data gathering. All events are sorted by event time and an
event set contains all active pointers during a single measurement. The pointer ID can
be interpreted as a stroke identifier, and the pointer number just counts from zero up
through all active pointers. Touch action contains the touch event reported from the
touch screen and operating system.



each pointer has now a unique start and end event this algorithm computes
unique identifiers for each stroke and each round r, given the output of the
previous algorithm. This allows us to compare strokes later on directly.
    In a nutshell, for the identification we select all events with the same pointer
identifiers and sort them according to the event time ν. Afterwards all events
that take place between TOUCH DOWN or TOUCH POINTER DOWN event action and
their respective up event actions with the same pointer ID are accounted as
one stroke. The event TOUCH DOWN marks the first touch of the first pointer of a
gesture or after a stroke gap. TOUCH POINTER DOWN is fired when further pointers
join the set of active pointers while others are already active. The output of this
algorithm is T (1) .


4.2.3   Motion Data Snuggling
We state that a gesture does start with the first touch and ends with the last
touch release before finishing the round by tapping on a button on the screen.
Therefore, we want to cut off the unnecessary motion data before the first touch
event and after the last one for each round. Note that this preserves the motion
data for stroke gaps, since they are between strokes, that is only motion data is


                                          16
generated but no touch data. We call this process snuggling, because the motion
data gets close to the touch data and encloses it. Hence, this algorithm computes
the cutoff for the motion data sets G (0) and A(0) . Observe that before touching
a device and after releasing a pointer, i.e., finger, from the device it moves a bit
by the finger motion and the hand holding the device depending on the relative
positions. Therefore we introduce two time offsets as system parameters, POS
for the offset at the beginning and POE for the offset at the end of a round.
Previous work [22] and our tests yield the following values in milliseconds:

                          POS := 150,        POE := 100.

Applying the cutoff yields the data sets G (1) and A(1) .


4.2.4    Motion Fusion Set Computation
Both the gyroscope and accelerometer sensors report data in their own speed
and hence are not synchronized, often they even may “stutter” a bit. Even if we
tell the sensors to report data every five milliseconds some background processes
may keep the CPU busy, such that a positive or negative delay may occur until
the data is handed over to SMAUG. The task of this algorithm is to merge
motion events of G (0) and A(0) that belong together, i.e., have happened at the
same time. To this end, we assume that the workload of the CPU is at a minimum
such that in the majority of the cases there is a low delay in the reporting of
events that took place at the same point in time. The algorithm searches for
value pairs and aligns these data sets which results into a single synchronous
data set of motion events. Obviously this may result into the situation that
some motion events are discarded. However, this is not a problem due to the
huge data amount gathered.
    We create a new motion data set F (1) containing these fusion values. An
event in this set consists of the following eight values: The gesture identifier,
round number r, three values from each motion sensor, and the associated event
timestamp.


4.3     Feature Extraction

After post processing of the data sets, the next step is to extract characteristic
features. A feature can be any value computed from an original data set. Though
it should represent a property of the input, for example the start point coordi-
nates of a stroke or the variance of the pointer size. For an example of feature
comparisons, see Figure 2. These features can later on be compared to feature
values of other data sets. SMAUG employs not only the features of single sen-
sors, but fuses them in different ways. The motion sensors are synchronized and
interpreted as a single sensor, without excluding individual sensor features. Data
from the touch screen is structured in a way which allows for the extraction of
stroke dependent features for each single stroke in addition to features applying
to the whole gesture at once. Merging everything together yields an entangled


                                        17
net of features allowing to distinguish between users. The main algorithm for
this step is denoted by

(T (1) , T (2) , T (3) , G (2) , A(2) , F (2) )r ← FeatureExtraction((T (1) , G (1) , A(1) , F (1) )r ).

It applies the following six subsidiary algorithms to the corresponding data in
each round r. As the names indicate the first three algorithms operate on touch
related data while the remaining three compute on motion data.


           1,000                       1,000                      1,000
             500                         500                        500

           5,000                       5,000                      5,000
           2,500                       2,500                      2,500
               0                           0                           0
               1                           1                           1
             0.5                         0.5                         0.5
               0                           0                           0
                       User U1                    User U2                        User U3

Fig. 2. Each column represents the inputs of one of three different users (U1 , U2 , or U3 )
for the gesture Y. A graph is generated for each user and feature y-coordinate (top blue
graphs), velocity in pixel/second (middle red graphs), and curvature (bottom green
graphs). While the Y-shape is recognizable in the top graph, the other graphs differ
more per user, and a combination of all three features allows a user identification with
high probability. To provide distinguishability between more users, or to prevent an
impostor, more features will be required for a gesture – SMAUG employs 320 different
features.




4.3.1     Touch Event Features
This algorithm computes within a round r the features curvature C ∈ R and
direction D ∈ R for each stroke s of a touch event. Additionally, velocity and
acceleration in x and y direction are computed as V (X) , V (Y ) , A(X) , A(Y ) ∈ R.
    Let s be an arbitrary stroke within the considered round r and let Is be the set
of indexes that refer to vectors of T (1) that belong to this stroke. In the following,
                                                                                (1)
let φ ∈ {x, y, ν} =: F denote a feature of (T (1) )r . Let s[φ] := i∈Is (Ti [φ])r
                                                                       S
be a short notation for all feature entries of feature φ in stroke s. Selecting a
specific entry j of s[φ] is denoted by sj [φ].
    The curvature Ci at point (si [x], si [y]), 1 ≤ i ≤ |s[x]|, can be computed as

                                 4Φ(yi )Θ(si [x]) − 4Φ(si [x])Θ(si [y])
                        Ci :=                                      3/2
                                                                             ,
                                      (Θ(si [x])2 + Θ(si [y])2 )


                                                  18
where Φ(si [φ]) := si+1 [φ] − 2si [φ] + si−1 [φ] and Θ(si [φ]) := 12 (si+1 [φ] − si−1 [φ])
with φ ∈ {x, y}. The direction at point (si [x], si [y]), is defined as
                                                 
                                si+1 [y] − si [y]
               Di := arctan                         ,   0 ≤ i < |s[x]|.
                                si+1 [x] − si [x]
Velocity in x- and y-direction is computed by

     (X)        si [x] − si−1 [x]        (Y )        si [y] − si−1 [y]
   Vi      :=                     ,    Vi       :=                     ,   1 ≤ i ≤ |s[x]|.   (1)
                si [ν] − si−1 [ν]                    si [ν] − si−1 [ν]
Acceleration in x- and y-direction of a touch event is defined as follows
                   (X)       (X)                        (Y )      (Y )
     (X)         Vi     − Vi−1           (Y )         Vi − Vi−1
   Ai      :=                     ,    Ai       :=                     ,   2 ≤ i ≤ |s[x]|.   (2)
                si [ν] − si−1 [ν]                    si [ν] − si−1 [ν]
These values are a good representation of the user’s behavior as they show how
fast and in which direction the user moved her fingers straight or curved. Other
gesture recognition algorithms also employed the same features with reliable
results, cf. [33]. We store these features as new properties into T (1) .

4.3.2      Touch Round Features
Next, we compute global touch features for round r. These features include the
number of records from the touch screen, frequency of these records, maximum
number of pointers touching the screen at the same time, overall length traveled
in pixels, overall time elapsed in nanoseconds, number of strokes, x- and y-
coordinates of the center of the gesture box, and the width and height of the
gesture box. Here, a gesture box is the smallest rectangle including the gesture
on the touch screen. With overall length or time we denote the sum of lengths or
time of all strokes of this round, respectively. We store these features in the new
data set T (2) where each row consists of the gesture identifier, round number r,
and the feature values mentioned above. These values have also been used by
signature recognition algorithms [11].

4.3.3      Touch Stroke Features
This algorithm computes a number of features for each stroke within round r. As
these computations are done separately for each stroke it is sufficient to explain
these for one stroke s. In the following, let

                    φ ∈ {x, y, P, S, C, D, V (X) , V (Y ) , A(X) , A(Y ) } =: F

denote a feature of (T (1) )r .
   First, the minimum and maximum values for each feature sequence are com-
puted as min(s[φ]) and max(s[φ]). Then the following features of the stroke s
are computed: length in nanoseconds, length in pixels, x- and y-coordinates of
the start- and endpoint, start time and end time, the sum of pixel length in x


                                                 19
and y direction respectively, width and height of the stroke box, and x- and y-
coordinates of the center of the stroke box. As a measurement of central tendency
we compute the arithmetic mean AM and root mean square RMS as

                                        1 X
                          AM (s[φ]) :=            si [φ],                         (3)
                                       |Is |
                                             i∈Is
                                       s
                                             1 X              2
                         RMS (s[φ]) :=                (si [φ]) .                  (4)
                                          |Is |
                                                     i∈Is


Based on these values we can now compute dispersion metrics such as the vari-
ance VAR, standard deviation StDev, and the mean absolute deviation MAD:

                                    1    X                        2
                  VAR (s[φ]) :=               (si [φ] − AM(s[φ])) ,               (5)
                               |Is | − 1
                                         i∈Is
                               p
                StDev (s[φ]) := VAR (s[φ]),                                       (6)
                               s
                                       1    X
                MAD (s[φ]) :=                    |si [φ] − AM(s[φ])|.             (7)
                                  |Is | − 1
                                                 i∈Is


Next, we compute the third and fourth central moment skewness Skew and kur-
tosis Kurt of each feature sequence of the stroke, respectively. Skewness describes
the asymmetry of a probability distribution and kurtosis is a measurement of
peakedness or flatness of a curve. For example, this can be helpful to distinguish
between the gestures of a letter “U” and a letter “V”. We set
                                                                        3
                                    1    X           si [φ] − AM(s[φ])
                Skew (s[φ]) :=                                                ,   (8)
                               |Is | − 1                 StDev (s[φ])
                                          i∈Is
                                                                        4
                                      1    X         si [φ] − AM(s[φ])
                Kurt (s[φ]) :=                                                .   (9)
                                 |Is | − 1               StDev (s[φ])
                                          i∈Is


In the realm of biometric data processing, this set of features is also used for
measuring human behavior [16]. To complete these computations, we finally
determine the stroke length in pixels and nanoseconds in percentage of the round
length, see Section 4.3.2. We summarize our results in a new data set T (3) where
each row consists of the gesture identifier, the round number r, the stroke number
s, and each of the computed features.


4.3.4   Motion Event Features
The first and second order numerical differentiation are computed by this algo-
rithm for round r and all sensor values of G (1) , A(1) , F (1) . The computations are
very similar to equations (1) and (2). These values are joined to the respective
data set.


                                           20
4.3.5    Motion Round Features
For each round r and each motion sensor data set G (1) , A(1) , we have nine
feature sequences per motion event: three sensor values, three values for the
first order numerical differentiation, and three values for the second order nu-
merical differentiation. To shorten the notation, we represent each sequence by
S[φi ] := (S (1) [φi ])r , where φi ∈ {sensorvali , sensorval0i , sensorval00i }. The num-
ber of primes denote the order of the numerical differentiation and sensorvali
denotes the sequence of the i-th sensor axis, 1 ≤ i ≤ 3. As in the short notation
of strokes, by Sj [φi ] we denote the j-th element of the sequence S[φi ]. Note that
S (1) does not include F (1) for this algorithm and merely represents either data
set G (1) or A(1) .
    In this algorithm we use the same features as for the touch stroke data algo-
rithm to compute the respective features for each sequence S[φi ], see equations
(3)-(9). These features are often used for accelerometer sensor data evaluation
[16]. We also compute the correlation value between the three axis pairs of each
motion sensor. Next, we compute four matrix norm values over {φ1 , φ2 , φ3 } for
the same differentiation and round r: 1-norm || · ||1 (maximum absolute column
sum of the matrix), ∞-norm || · ||∞ (maximum absolute row sum of the matrix),
Frobenius norm || · ||F (square root of the squared sum entries of the matrix),
and squared `2 -norm || · ||2`2 as follows:
                                                              
                                            |S[φj ]|
                                             X
                           || · ||1 := max          |Si [φj ]| ,
                                        1≤j≤3
                                                     i=1
                                                                    
                                                        X3
                      || · ||∞ :=          max            |Si [φj ]| ,
                                        1≤i≤|S[φ1 ]|
                                                            j=1
                                   v
                                   u|S[φ1 ]| 3
                                   u X X
                       || · ||F := t           Si [φj ]2 ,
                                              i=1   j=1
                                        3
                                        X
                    || · ||2`2 [i] :=         Si [φj ]2 ,         1 ≤ i ≤ |S[φ1 ]|.
                                        j=1

Finally, we compute the minimum, maximum, and arithmetic mean from the
vector of the squared `2 -norm || · ||2`2 . These values were chosen because they
showed good results in classifying user motion on touch screen devices, cf. [22].
The frequency of the sampling is not stored on purpose as this depends on
external factors like CPU workload which may differ from gesture to gesture.
All resulting feature values are stored in G (2) and A(2) , respectively, together
with the gesture identifier and the round number r.

4.3.6    Motion Fusion Features
As the two previous algorithms derived features for each sensor data separately
the aim of this algorithm is to determine correlations between both gyroscope


                                                     21
and accelerometer data sets in F (1) . We compute the angle between both motion
sensor vectors and its rate of change, also for the first and second order numerical
differentiations, and the Pearson correlation coefficients PCC between all nine
pairs of the individual sensor axis. Let G[i] := (F (1) [gyroi ])r be the sequence
of all gyroscope data of the i-th axis in the fusion data set F (1) . Furthermore,
we define similarly to previous notation Gk [i] as the k-th value of the sequence
G[i]. We also set G̃k [i] := Gk [i] − AM (G[i]) and analogously define A[j], Ak [j],
and Ãk [j] for the accelerometer data. Note that it holds |G[i]| = |A[j]| for any
combination of 1 ≤ i, j ≤ 3 since there is the same amount of sensor data for
each event in the fusion data set. Then, the PCC coefficients are computed as
                                            P|G[i]|
                                                k=1 G̃k [i]Ãk [j]
        PCCi,j :=                  q                          q                        ,
                                           1
                                                P |G[i]|             1
                                                                          P|G[i]|
                    (|G[i]| − 1)       |G[i]|−1   k=1 G̃k [i]    |G[i]|−1  k=1 Ãk [j]


for 1 ≤ i, j ≤ 3, where PCCij describes the PCC values between axis i of the
gyroscope and axis j of the accelerometer. These features have been chosen
because they performed well in our tests and have been used before in similar
settings [22]. The six angle related values are added to F (1) while the nine
correlation values are added to a new set F (2) together with a gesture identifier
and round number r.


4.4     Template Generation

For the verification process later on, one requires a set of features, called a
template, that is efficiently comparable for two different gestures. The template
generation algorithm is given as follows
                                                                                
  (T , S) ← TemplateGeneration M, (T (1) , T (2) , T (3) , F (1) , S (2) )r 1≤r≤P .
                                      
                                                                                           E



The template (T , S) contains all previously computed values required for a ges-
ture comparison and additionally determines limits for the feature values. Fur-
thermore, meta data are added in order to label the template correctly. The
algorithm consists of six sub-algorithms that are described below. Each of them
computes five values: minimum, maximum, standard deviation, median, and
arithmetic mean value for each feature sequence.


4.4.1    Touch DTW Template
The algorithm of Section 4.2.2 yielded all involved strokes. Thus, strokes from
different rounds can be associated to each other. This, however, requires that
the strokes are always input in the same order. The goal of this algorithm is to
determine for each stroke of the gesture the “best” stroke in terms of deviation
from the same stroke of the other rounds. It is based on DTW and can be seen as
an extension for multi-touch and multi-stroke recognition of the technique used
in [19]. The input for this algorithm is T (1) . In a nutshell, it computes for each


                                               22
round and each stroke the distance to the same stroke in the other rounds using
DTW. Therefore, the best stroke is represented by a sequence of the round with
the shortest DTW-distance compared to all other rounds.
    For each stroke s and each feature φ ∈ F we denote the sequence of its
feature values within this stroke as in Section 4.3.1, that is by s[φ]. We denote
by (s[φ])r the respective feature values of stroke s in round r. Next, we compute
the distance between feature sequences of all rounds by

                            dφr,r0 := DTW ((s[φ])r , (s[φ])r0 )

for each different pair of rounds 1 ≤ r < r0 ≤ PE . We also compute the
average of P  the distance for each round r and feature to all other rounds by
                 PE
aφr := P1E j=1      dφr,j . Afterwards, the values of a are normalized by dividing
them by the maximum m := maxr=1,...,PE (aφr ) of aφr over all rounds, resulting
in aφr := aφr /m. Next, the normalized distances are summed up and the result is
                                                            1
                                                              P|F| φi
divided by the number of features which yields dr := |F|        i=1 ar , φi ∈ F. This
allows determining the “best” stroke instance, i.e., the round where on average
the distances of the stroke      are the smallest compared to all other rounds, by
r? := minr=1,...,PE dr . Using the values computed before we can now deter-
mine a kind of “faultiness” of the other rounds in respect to the best round r?
for each feature φ ∈ F corresponding to the value dφr? ,r for each round r 6= r? .
Now we know how “bad” a touch sequence of the user may be, depending on the
amount of faultiness. Consequently, for each feature φ the following five values
are computed:
                                                                      
           φ                             φ                         φ
  min     dr? ,r               , max    dr? ,r         , StDev    dr? ,r           ,
               r=1,...,PE                                r=1,...,PE                               r=1,...,PE
                                                                                           
           Median        dφr? ,r                        , and AM           dφr? ,r                    ,
                                       r=1,...,PE                                    r=1,...,PE

where Median(I) denotes the median of a sequence I. The terms StDev and
AM are defined in equations (6) and (3), respectively. Observe that these values
belong to each stroke instead the whole gesture which allows us to use multi-
touch events and multi-strokes. Finally, we store these values together with a
gesture identifier, stroke number s, and best round r? in a new template data
set T (4) .

4.4.2   Touch Gesture Template
Let F1 := {maximum number of pointers at the same time, number of strokes}
and F2 := {records, overall length in pixels, overall elapsed time in nano seconds,
x coordinate of the gesture box center, y coordinate of the gesture box center,
box width, box height} for each round r, see Section 4.3.2. The number of strokes
and the maximum number of pointers at the same time are crucial features for
defining a gesture. A deviation in these features results in a completely different
structured gesture. Therefore we call them strong features and compute the


                                                        23
template value as the rounded down value of AM of the strong features over all
rounds. Regarding F2 , we compute the five template values minimum, maximum,
standard deviation, median, and arithmetic mean of T (2) [φ] for each φ ∈ F2 . We
store all computed values in the new template data set T (5) together with a
unique gesture identifier.


4.4.3   Touch Stroke Template
For each stroke s and each feature φ ∈ F of T (3) , we compute the minimum,
maximum, standard deviation, median, and arithmetic mean of the sequences
T (3) [φ]. The results are stored together with a unique gesture identifier and
stroke s in the template data set T (6) .


4.4.4   Motion Round Template
For each of the two motion sensors, this algorithm takes all features described
in Section 4.3.5 of G (2) and A(2) , and computes for each feature sequence its
minimum, maximum, standard deviation, median, and arithmetic mean. Finally,
these values are stored together with a unique gesture identifier in the respective
template data sets G (3) and A(3) .


4.4.5   Motion Fusion Template
This algorithm operates on the fusion data set F (2) and the features computed
in Section 4.3.6. For each feature sequence, the minimum, maximum, standard
deviation, median, and arithmetic mean are computed. The resulting values are
stored together with a unique gesture identifier in the template data set F (3) .


4.4.6   Motion DTW Template
This algorithm works analogue to Section 4.4.1. However, we now employ the
fusion data set F (1) instead of touch data. We again get the best round r?
for all distances measured for each feature. Finally, we compute the minimum,
maximum, standard deviation, median, and arithmetic mean values for each
feature distance sequence. Afterwards, we store these values together with a
unique gesture identifier and best round r? in the template data set F (4) .

   Finally, the algorithm TemplateGeneration returns the template (T , S), where
                                      
T := ∪Pr=1
        E
           ∪3i=0 (T (i) )r ∪ ∪6i=4 T (i) ,
                                                                               
S := ∪Pr=1
        E
             ∪2i=0 (G (i) )r ∪ (A(i) )r ∪ ∪2i=1 (F (i) )r ∪ G (3) ∪ A(3) ∪ ∪4i=3 F (i) .

Please note that we implicitly also store a gesture identifier derived from meta
data M in the template.


                                         24
4.5     Computing Individualized Weights

Given the template as described above, the verification process could work as
follows: given a new input from some user U∗ , just check if it is sufficiently similar
to the templates. Not surprising, some features will be more characteristic than
others. However, our experiments showed that the expressiveness of features also
depends on the user and the entered gesture. In other words, each feature has a
different relevance for each user, gesture, and stroke. For example, for a user U1
the x- and y-coordinates could be more important than the velocity of drawing
the gesture since the user’s speed of performing the input varies while he is quite
accurate in reproducing the gesture. But for another user U2 , it may just be the
other way around.
    This makes it necessary to introduce and assign to each feature its feature
weight, expressing the level of relevance for the authentication process. Therefore,
the feature weights cannot be some global system parameters. In fact, SMAUG
identifies dynamically these weights during the enrollment phase depending on
the stability of the input. On top of these individual weights we also introduce
feature tiers. Our experiments showed that across all users, certain groups of
features always perform slightly better for honest user recognition than other
features. However, latter are getting useful when preventing strong adversaries
as allowed by our adversary model, see Section 2.4. A feature tier represents
a second weighting for a class of similar features and each feature is assigned
exactly to one tier. Based on our experiments, we decided to have three tiers in
total. In comparison, feature weights are computed individually, and feature tier
weights are pre-computed values based on our experiments. Summing up, the
weight computation algorithm is denoted by

                     (W, IF , IW ) ← ComputeWeights (T , S)

which takes the gesture template as input and outputs a set of weights W and
also two indicators IF , IW . It dynamically computes the feature weights for a
gesture and each of its strokes while taking the global feature tier values into
account. The algorithm can be described briefly as follows: by cross-validation
we compare the gestures template to each gesture round input performed by
the user and store the deviations in a novel fault container E. Afterwards, we
compute the feature weights based on the information in the error container E.
The feature weight computation depends on the number of overall rounds during
the enrollment phase PE , the more data the better the weight estimation.


4.5.1    Internal Cross-Validation
The cross-validation step performs comparison checks for each of the original user
inputs {(T (0) , G (0) , A(0) )r }1≤r≤PE to the gesture template (T , S) and outputs
the fault container E.
   There are three possible types of comparison which we denote by C :=
{LB, UB, EQ}, where LB stands for “lower bound”, UB for “upper bound”, and


                                          25
EQ for “equality”, respectively. The comparisons are defined as follows:
                                       (
                                        0, v < 21 (a + m) − s,
                    LB(v, a, m, s) :=                                           (10)
                                         1, else,
                                       (
                                         0, v > 21 (a + x) + s,
                     UB(v, a, x, s) :=                                          (11)
                                         1, else,
                                       (
                                         0, v = w,
                         EQ(v, w) :=                                            (12)
                                         1, else.

Here, v and w refer to a feature value, and a to the arithmetic mean, m to the
minimum, x to the maximum, and s to the standard deviation stored in the
according template. If a comparison fails, the output will be 1, and otherwise 0.
The form of these comparisons are a result of our extensive testing phase. This
gives a very good estimation of the deviation in the “environment” of features
caused by a user’s behavior. The kind of comparison performed depends on
the feature, but the result is always binary: either a comparison is passed or
otherwise failed. Features failing a comparison get stored in E together with
their comparison type c. The cross validation algorithm consists of the following
seven subsidiary algorithms, which are performed for each r = 1, . . . , PE .

4.5.1.1   Strong Features Check
This algorithm compares the strong features of the gesture, that is “number of
strokes” and “maximum number of pointers at the same time”. Both features are
denoted by the set F1 , see Section 4.4.2. The comparison is done as an equality
check between data set T (2) (see Section 4.3.2) and template data set T (5) (see
Section 4.4.2). Please observe, that T (2) [φ] is a single value. If a feature is not
the same in both sets, i.e., the test
                                              
                      EQ T (2) [φ] , T (5) [φ] , φ ∈ F1 ,
                                     r

as defined in equation (12) fails and equals 1, the feature φ and round r together
with comparison type EQ get stored in E.

4.5.1.2   Touch Gesture Check
This algorithm checks if the corresponding features of the set (T (2) )r do not
“deviate too much” from the template data set. That is, each feature φ ∈ F2
is verified according to a lower and upper bound, where F2 is defined in Sec-
tion 4.4.2. Hence, the comparisons from equation (10) and equation (11) are
parametrized as
                                                               
       LB T (2) [φ] , T (5) [AMφ ], T (5) [minφ ], T (5) [StDevφ ] ,
                  r                                            
       UB T [φ] , T (5) [AMφ ], T (5) [maxφ ], T (5) [StDevφ ] , φ ∈ F2 .
               (2)
                     r



                                         26
The notation T (5) [AMφ ] refers to the arithmetic mean of the feature φ stored in
the data set T (5) , and analog minimum, maximum, and standard deviation. If
a test fails, i.e., LB = 0 or UB = 0, the feature φ, round r, and bound (LB or UB)
are added to the fault container E.

4.5.1.3   Touch Stroke Check
For each stroke s in T (6) we compare the feature values φ ∈ F with the ones
from the value of the set (T (3) )r . This is done by checking the lower and upper
bounds using equations (10) and (11), respectively. The single feature value is
given by (T (3) [φ])r while the according values of AM, min, max, and StDev are
provided directly by T (6) [φ], that is
                                                              
      LB T (3) [φ] , T (6) [AMφ ], T (6) [minφ ], T (6) [StDevφ ] ,
                   r                                          
      UB T [φ] , T (6) [AMφ ], T (6) [maxφ ], T (6) [StDevφ ] , φ ∈ F.
               (3)
                      r

On failing a test, the respective feature φ, round r, comparison type, and stroke
s are added to the fault container E.

4.5.1.4   Touch Stroke DTW Check
This algorithm takes the set (T (1) )r? as an input, where r? denotes the best
round, see Section 4.4.1. The comparison data set is (T (1) )r . The following two
steps are done for each stroke s of the gesture.
    First, we check for two features whether they are equal, namely F3 :=
{“pointer identifier”, “pointer numbers”}. The values of F3 are also treated as
strong features. Hence, if they are not equal in (T (1) )r? and (T (1) )r for the given
stroke s, they get added to the set E together with EQ. Second, the DTW dis-
tances of feature sequences are considered. Now the “best” round of the gesture
inputs gets compared to all other data sets, i.e., (T (1) [φ])r? and the data set
(T (1) [φ])r which yields the DTW distance. Then, this is compared to the values
of T (4) as
                                                                    
            UB dφr? ,r , T (4) [AMφ ], T (4) [maxφ ], T (4) [StDevφ ] , φ ∈ F.

If the distance is not below the upper bound given by equation (11), it is con-
sidered as an error and the feature is inserted into E together with comparison
type UB, round r, and stroke s. Note that we do not have to check for a lower
bound as it holds for at least one test data that one feature perfectly matches,
i.e., yielding a distance of 0.

4.5.1.5   Motion Gesture Check
The motion gesture check uses a set of features which we denote by FMG . A
brief description of its 90 features can be found in Section 4.3.5. For each the
gyroscope and the accelerometer data, this algorithm will check the features of
the set FMG in comparison between the values of G (2) and G (3) , as well as A(2)


                                          27
and A(3) . Seven features are computed for each axis value and its first and second
numerical differentiation. Also the correlation between each axis pair as well as
four matrix norm values of the axis values and differentiations are part of this
set. Then, it performs for each motion sensor and each feature the checks given
by equations (10) and (11) as follows:
                                                               
     LB        G (2) [φ]
                    , G (3) [AMφ ], G (3) [minφ ], G (3) [StDevφ ] ,
               r                                               
     UB G (2) [φ] , G (3) [AMφ ], G (3) [maxφ ], G (3) [StDevφ ] ,
               r                                                
     LB A [φ] , A(3) [AMφ ], A(3) [minφ ], A(3) [StDevφ ] ,
          (2)

               r                                                 
     UB A [φ] , A(3) [AMφ ], A(3) [maxφ ], A(3) [StDevφ ] ,
          (2)
                                                                           φ ∈ FMG .
                               r

The feature φ, round r, and comparison type are added to E whenever a check
fails.

4.5.1.6    Motion Fusion Check
For this check we are comparing fusion data to the fusion template data. The
feature set is given by FMF and includes correlation between all nine axis pairs
of gyroscope and accelerometer. Furthermore, the angle between both motion
vectors, its rate of change, velocity, velocity rate of change, acceleration, and
acceleration rate of change complete this feature set. Using equations (10) and
(11), we again check the lower and upper bounds by comparing the original value
to a combination of template values as
                                                            
     LB        F (2) [φ]
                 , F (3) [AMφ ], F (3) [minφ ], F (3) [StDevφ ] ,
            r                                               
     UB F [φ] , F (3) [AMφ ], F (3) [maxφ ], F (3) [StDevφ ] ,
          (2)
                                                                           φ ∈ FMF .
                               r

Feature φ, round r, and comparison type are added to the fault container E if a
test fails.

4.5.1.7    Motion Fusion DTW Check
In this algorithm we employ the feature set FMD given by the angle between both
motion vectors, its rate of change, velocity, velocity rate of change, acceleration,
and acceleration rate of change. Next, we perform the upper bound check over
(F (1) )r , (F (1) )r? , and F (4) as
                                                                  
          UB dφr? ,r , F (4) [AMφ ], F (4) [maxφ ], F (4) [StDevφ ] ,   φ ∈ FMD .

The feature φ, round r, and comparison type UB are added to the fault container
E if a test fails.


                                             28
4.5.2   Weight Calculation
This algorithm takes the fault container E as input and outputs the weight set W
for all features considered, that is for both the gesture as a whole and also each
stroke. Recall that the subsidiary algorithms of the internal cross-validation algo-
rithm performed one or two checks, depending on the feature, see Section 4.5.1.
Therefore, for some features up to two faults may occur, namely failing on both
comparisons LB and UB. In contrast, tests for DTW related features and strong
features perform only a single comparison, namely UB or EQ, respectively. We call
a pair in E consisting of a feature φ and an associated comparison type c ∈ C
comparison feature φ∗ := (φ, c) ∈ E. These are the values which have been added
in the previously performed internal cross-validation algorithm.
    Now, we are going to explain how for each φ∗ a corresponding weight is
computed. In principle, the weight is defined as the frequency that φ∗ did not
fail, multiplied by a constant that depends on the tier of φ∗ . More precisely, let
F∗G denote all comparison features for the global gesture independent of strokes,
and F∗s all comparison features exclusively for a single stroke. Furthermore, let
ns denote the number of all strokes in this gesture. For a fault container E, a
comparison feature φ∗ ∈ F∗G and a round r ∈ [PE ], the term E0 (φ∗ , r) is equal to
1 if the corresponding fault occurred in E with respect to round r, and is equal
to 0 otherwise. This is according to the outputs of the three comparisons which
output 1 if a comparisonPfails, see equations (10), (11), and (12). Moreover,
we define by E0 (φ∗ ) :=                      ∗                                    ∗
                                 r∈[PE ] E0 (φ , r) the number of occurrences of φ in
         ∗         ∗                         ∗
E for φ ∈ FG . In other words, E0 (φ ) yields how often the feature φ did not
pass a comparison check during the internal cross-validation phase. Observe
that if the fault container E contains only one round, e.g., as it is the case
for the verification phase later on, it holds E0 (φ∗ , r) = E0 (φ∗ ). Likewise, we
               ∗
define Es (φP    , r) for a comparison feature φ∗ ∈ F∗s and a stroke s ∈ [ns ], and set
      ∗
Es (φ ) := r∈[PE ] Es (φ∗ , r).
    As explained in Section 4.5, all features are partitioned into tiers, where one
feature belongs to exactly one tier. A tier is a weight control for the features and
their respective weights. We consider three different tiers T 1, T 2, and T 3. By
Tφ we denote the tier T of feature φ, which represents one of the system values
PT 1 , PT 2 , or PT 3 . We determined in tests which features should be assigned to
which tier. Because of space restrictions, we cannot list the partitioning here. In
general, the more directly the features are derived from user behavior, the higher
the tier. We set the following system parameters, resulting from our experiments:

                     PT 1 := 0.5,      PT 2 := 2,     PT 3 := 4.

Then, for the comparison features, their weights are defined as follows:

                               E0 (φ∗ )
                                       
                   ∗
                ω0φ := Tφ 1 −             , φ∗ ∈ F∗G ,                            (13)
                                 PE
                               Es (φ∗ )
                                       
                   ∗
                ωsφ := Tφ 1 −             , φ∗ ∈ F∗s , s ∈ [ns ].                 (14)
                                 PE


                                          29
We define by IF [r] a fault sum indicator for round r. This is the sum of all
comparison faults in fault container E which occurred during round r. Similarly,
a weight sum indicator IW [r] contains the sum of all weights for all faults of
round r. More precisely, this is computed as follows:
                                          ns X
                          X          ∗    X              ∗
             IF [r] :=            ω0φ +                ωsφ
                         φ∗ ∈F∗
                              G           s=1 φ∗ ∈F∗
                                                   s
                                                        ns X
                          X                      ∗      X                                ∗
            IW [r] :=             E0 (φ∗ , r) · ω0φ +                Es (φ∗ , r) · ωsφ
                         φ∗ ∈F∗
                              G                         s=1 φ∗ ∈F∗
                                                                 s


Next, we compute the arithmetic mean and deviation of both indicators over all
rounds according to equation (3) and equation (6), respectively. This yields the
values AM(IF ), AM(IW ), StDev(IF ), and StDev(IW ). Finally, we compute the
global fault sum indicator IF and weight sum indicator IW as

                            IF := AM(IF ) + StDev(IF ),                                      (15)
                            IW := AM(IW ) + StDev(IW ).                                      (16)

In other words, these formulas yield the mean value of overall errors occurred
and also their according weight sums. The ultimate goal for these values is to
give a numeric representation of the quality of the user inputs which will make
it comparable to new inputs, i.e., authentication attempts. We have chosen the
combination of arithmetic mean and standard deviation as the best performing
one. For example, using only the arithmetic mean is not robust enough, since
“small” outliers are excluded, but our extensive experiment phase showed that
they are definitely part of a legitimate user. On the other hand, if we would only
use the maximum here, even the worst of all inputs of a user is still valid. Simi-
larly, a similar equation as in the comparison bounds, i.e., employing minimum
or maximum additionally, is still too forgiving for the later verification phase.
    We store IW , IF , and all weights ω of the comparison features (see equa-
tions (13) and (14)) in the set denoted by W.

4.6   Thresholds
Thresholds are crucial to distinct between a valid authentication try and an
impostor later on. We now describe the algorithm

                             (Θ1 , Θ2 ) ← Thresholds(W).

Threshold Θ1 is required for the weight sum of feature faults occurred and Θ2 is
a threshold for the number of feature faults occurred. Their values depend on the
number of strokes ns of the gesture as well as the weight sum indicator IW and
fault sum indicator IF , computed in equation (16) and (15), respectively. Both
values has been stored in the set W. Note that the more strokes a gesture has, the
more faults may occur. Therefore, the number of strokes ns directly influences


                                               30
the number of faults and at the same time their weights. The thresholds are
computed as follows:

                         Θ1 := IW PW ∗ + PW + (1 + ns ),                       (17)
                         Θ2 := IF P  F∗   + PF + (1 + ns ).                    (18)

We introduce the four security parameters of SMAUG here, which allow us to
cope with the different modes, see Section 3.3. These are PW ∗ and PF ∗ as mul-
tiplicative values for each threshold, while PW + and PF + are additive values for
each threshold. If a background image is used, the security parameters should be
different than without picture, because also the attacker can remember the ges-
ture more easily. Without a picture the user has no orientation where exactly to
hit the screen, therefore the values should lead to more forgiveness. Furthermore,
the thresholds also depend on the structure of the gesture, i.e., if it was single-
or multi-touch. This is due to the fact that single-touch is easier to replicate. In
our tests we computed the values shown in Table 3.


         Background Image         Single-Touch                Multi-Touch
                               PW ∗ = 2.5, PW + = 6, PW ∗ = 2.2, PW + = 4,
                 Yes
                                PF ∗ = 1.9, PF + = 6 PF ∗ = 2.1, PF + = 4
                               PW ∗ = 3, PW + = 7, PW ∗ = 2.5, PW + = 8,
                  No
                               PF ∗ = 2.1, PF + = 7 PF ∗ = 2.5, PF + = 8

Table 3. Default values of SMAUG’s security parameters, which depend on back-
ground image and maximum number of active pointers at the same time.




5   Verification Phase

In this section we will explain the verification phase in detail. We will stick to
the algorithmic description of Algorithm 3 and explain the five subsidiary algo-
rithms. In more detail, for each authentication try t, 0 ≤ t ≤ PV , the while loop
is executed until a valid input was made. As soon as the verification returns
true, the algorithm outputs the decision dec with value true. If the authenti-
cating user fails in all PV + 1 trials, the authentication fails completely. Then,
a fallback mechanism will handle the next steps. For a brief discussion on this
see Section 3.5. However, the value of PV should be chosen relatively small in
order to prevent repeated testing, but high enough for a reasonable usability,
such that a mistake by an honest user is not immediately punished. We think
that PV := 3 is a good value and set this as default. To summarize this, the
goal of the verification phase is to determine if an authentication attempt was
successful.


                                          31
5.1   Data Gathering & Post Processing
For an authentication attempt t, 1 ≤ t ≤ PV , the possibly unknown user U∗
                                                         f, Te (0) , Ge(0) , Ae(0) ). This
enters a gesture G* which results in the test data sets (M
is done exactly the same way as described in Section 4.1. Then, post processing
is applied, which is in fact the same as in Section 4.2. This yields the test data
sets Te (1) and Se(1) .

5.2   Test Feature Extraction
This step consists of extracting features from Te (1) and Se(1) . Here, the previ-
ously presented algorithm FeatureExtraction from Section 4.3 can be employed
analogous. However, please note that in this case we only have one data set
instead of PE sets, since only one input round in given. In other words, for an
authentication attempt the gesture is performed once instead of PE times as in
the enrollment phase. Therefore we slightly alternate the algorithm and reduce
all feature computations to one single round. We also omit the output of Te (1) .
Next, we denote the final algorithm for test feature extraction by

             (Te (2) , Se(2) ) ← VerificationFeatureExtraction(Te (1) , Se(1) ).

This already yields all values required for a comparison of the test data set
features and the gesture template data sets computed in the enrollment phase.

5.3   Validation
By employing a single cross-validation, the validation algorithm computes the
number of all faults, being the features which do not fulfill requested bound
checks, and the weights of those faults. We compute

                  (I
                   g                       e (2) , Se(2) ), (T , S), W),
                    W , IF ) ← Validation((T
                        f

being the same algorithm as given in Section 4.5.1. In this case, the algorithm
yields a fault container eE. Then, the corresponding test fault sum indicator IgW
and test fault number indicator If  F are computed.  This  is accomplished  by the
formulas (16) and (15), respectively, but with the difference that the test fault
container Ee is used instead of E. It is important to note that the computation
of I
   g W uses the weights stored in W (see Section 4.5.2). This ensures that the
gesture G* is interpreted as an input of the legitimate user U. If it was the user
U, it will fit very well. But if an impostor tried to login as U, the weights will
mess up the indicator computation in the sense that the value is far off from the
one by the legitimate user U.

5.4   Verification
As a final step of the verification phase, we execute the algorithm

                         dec ← Verification(Θ1 , Θ2 , I
                                                      g W , IF ).
                                                            f


                                             32
The thresholds Θ1 , Θ2 have been computed in equations (17) and (18), and both
indicators I
           gW , IF in the previous section. We set the initial value of the binary
                f
decision variable dec to false. The comparison then is the test of the following
logical equation:
                                                    
                                      ?         ?
                         dec = IW ≤ Θ1 ∧ IF ≤ Θ2 .
                                  g          f


The output of dec is changed to true if the equation holds. This concludes the
verification phase and determines if the authentication attempt was successful,
i.e., U∗ = U and G* = G for any user and gesture.


6     Experiments

In this section we first describe our implementation and test setup for the par-
ticipants. Afterwards we give a discussion on the experiment results.


6.1   Implementation

We implemented SMAUG on a Nexus 5 device running on Android 5.1.1. Users
are registered in this app by unique identifiers. A user can create as many gestures
as desired and manage them in the app. To simulate an authentication process,
a user is able to choose any user known to the system and any of her gestures
during the login phase. Then she can try to log in and receives a positive (login
successful) or negative (login failed) reply as feedback. The data gathered by the
users were stored in text files and evaluated on a laptop (1.8GHz, SSD), to make
data handling for various testing processes easier. On this computer we used
a database to handle the data and executed all algorithms except for the data
gathering step, which was always running on the smartphone. This especially
allows for bulk evaluations of already performed authentication attempts, i.e.,
“old” input data already transferred onto the computer, even if alterations on the
algorithm were being made during development and the testing phase. A single
gesture needs about 1MB of space and a single authentication is evaluated in
less than 0.1 seconds, depending slightly on the size of the gesture.


6.2   Setup

For our tests, we acquired 23 test participants, ten female and thirteen male
persons, for 36 test sessions.1 Out of these, 18 participants had a background
in IT-security. The participants were divided into four groups. The first group
used gestures without a background image while the second one was using ges-
tures with a background image. The third and fourth group had the same task,
however they also performed delayed inputs, waiting from half an our up to a
week between the enrollment and verification phase. A background image helps


                                        33
(a) Input of gesture A with   (b) Support for multi- (c) User authentication
helpful background image.     stroke and multi-touch was successful.
                              gestures is support as
                              shown on the input of
                              gesture DC.
Fig. 3. Input of gestures A and DC (double circle) during the enrollment phase and info
message after successful authentication.




remembering and attacking the gesture, see Section 3.3 and Figure 3(a). For the
background images, they had the choice between a grid, a mountain view, a field
of flowers, and a popular band logo. Otherwise the choice between a uniformly
black and a blue background color was given. Each participant got a handout
describing step by step how the app works and how the tests should be done.
They were also guided in the first steps by us in order to assist with the system
or to answer their questions. Usually a participant started by entering some test
gestures aiming to understand the reaction of the system and SMAUG. Each
test involved both predetermined gestures and freely chosen gestures. With re-
spect to the predefined gestures, they had to enter the gestures described in
Table 4. For the free-form part, the participants were asked to enter a minimum
of two own gestures. Mostly letters have been entered, but also numbers, arrows,
swirly lines, geometric shapes, greek letters, and also 3D-gestures (moving the
device around while performing the gesture). This once more shows the diversity
in gesture generation.
    For the enrollment phase, a user entered the name of the gesture and after
an information screen, she started to input the first round of the gesture. When
the user has finished performing the gesture for the first round, she taps on the
1
    Each participant agreed on using his/her anonymized data in this project. Further-
    more our IRB staff is informed about this project.


                                          34
      Type        Gesture Name & Description
                   Upper case letters A (see Figure 3(a)), B, I, U, and V
                   the word OK
                   square (start in corner top left, draw clockwise)
      Single-touch tap-rhythm
                   L-Turn (write L, turn phone left by 90 degrees, write L, turn
                   phone right by 90 degrees)
                  Bar & stripe (drawing a second line while drawing another one)
                  DC (double circle; drawing two circles at the same time), see
                  Figure 3(b)
      Multi-touch
                  wave (three fingers, put one after another on the touch screen
                  and lift them in the same order)

                Table 4. Predefined gestures used in the experiments.




bottom right button “Next” in order to proceed to the next round. The name of
the gesture is displayed on the bottom left and the current and overall number of
rounds is shown in the bottom center, see Figure 3(b). After all ten rounds, the
data gathering procedure is finished and the user is back in the menu of all stored
gestures on this device. The verification phase is structured similar, except that
the user can input the gesture as often as he wants, each time yielding either a
successful or failed authentication, respectively.
    The gestures A, I, U, V, square, wave, double circle, and L-Turn, as well
as at least one free-form gesture were entered from the participants while sitting,
holding the device relaxed in portrait mode with one hand and entering with
the other hand. The gestures A, B, OK, rhythm, bar, and at least one free-form
gesture were entered while standing relaxed while performing the inputs.
    The participants also performed attacks on each other. Recall that we allow
an adversary to watch the user closely. The user explained how he entered a
gesture and immediately afterwards the attacker tried to impost the user. For
these attacks, we selected representative gestures. Letter I as the most simple one
consisting of one stroke and no curves, letter A as a one fairly easy to remember
but consisting of one to three strokes, L-Turn as a complicated one, and Bar
& stripe as a simple multi-touch gesture. Each user performed attacks on these
gestures of another user. The attacker always used the same body position as
the user, i.e., sitting or standing.


6.3     Results

Overall, we gathered 8180 data sets, each containing the meta, touch, and motion
sensor data. For successful user authentication (true positives), we counted the
number of login attempts which were necessary for the legitimate user to be
successful. 77% of all valid login attempts, including free-form gestures, were
recognized by SMAUG at the first try, 92% after the second try, 99% after the


                                          35
third try. Allowing a fourth login try, 100% of all login attempts were successful.
Since this includes the free-form gestures, each participant was able to login with
their self-created gesture. As expected, the success rate also depended on how
involved the gesture is. For example, using multi-touch gestures, the 100% rate
was already reached after two login attempts, see also Table 5 for an overview
for representative gestures. In total we registered 1490 valid login attempts.
    Next, we evaluated the attack scenarios (false positives) with respect to the
strong adversary model explained in Section 2.4. An impostor was detected in
overall 97% of all 610 impostor attempts. 1.3% of these impostor attempts were
due to irregular inputs of the gestures A and I. In the 3% of successful impostor
attacks, only a single one was reproduceable once, but failed to do so over a
longer time of some minutes.
    In most experiments, inspecting the measured data revealed that the adver-
sary could mimic some aspects of the gesture quite successfully but then failed
miserably on other features. These results are a strong indication for capabilities
of our algorithm. We have to stress though that this also depends on the level of
complexity of the used gesture. Most simple gestures like a single line are easier
to reproduce, as mentioned for gesture I. Fortunately, our experiments showed
that gestures which are a little bit more involved are already sufficient for suc-
cessful authentication. For example using the gesture A was already sufficient to
achieve a security level of 99.3% (being the probability the gesture A was not
attacked successfully within three attempts).
    Table 5 shows the results of authentication (True Positives Rate, TPR) and
attack (False Positives Rate, FPR) tests for four different, exemplary gestures.
The more complicated and unusual a gesture was, the more difficult it gets to
authenticate with this gesture. However, this holds also for the attacks which will
fail with a very high probability as given in the table. Finally, the experiments
showed that the attacks failed completely for the free-form gestures, while login
was still possible. We also conducted ten tests with a delay of half an hour



                                 TPR in %      FPR in %
                       Gesture
                                 1st 2nd 3rd 1st 2nd 3rd all
                          I     77    91 100 7      13   27   5
                          A     75    92 100 0       0    6   5
                         Bar    73    93 100 0       0    8   3
                       FreeForm 65    88 91 0        0    0   0
                          all     77 92    99   2    4    8   3

Table 5. TPR (True Positives Rate, successful logins) and FPR (False Positive Rate,
successful attacks) for different gestures. 1st, 2nd, and 3rd, denote the number of the
attempt. Further “all” is the summarized result of all attempts, up to 10. Overall after
the third login attempt 98% of users were authenticated while only 3% of all attacks
were successful.



                                          36
between the enrollment and the verification phase. Remarkably, the participants
who have previously performed another test got authenticated in 95% of all
login attempts using a background image, and 90% without background image
after a maximum of three login attempts. Participants who did this test first,
were authenticated in 85% of the login tests with a background image, and 80%
without background image. As a consequence, we see that if one gets used to
a gesture as secure authentication method, the probability gets higher that the
authentication will succeed in the first place.
    An interesting study was to compare the same gesture from the same user in
different positions, i.e., sitting and standing. We tested this with the gesture A.
If the users did the enrollment phase while sitting and the authentication while
standing, 89% could login after at most three attempts, which was reproducible
for most of the participants in later login attempts. Vice versa, only 11% were
able to login while sitting, only a few managed to login after the third attempt.
When the gesture has been entered using a background image, only 10% of the
users could authenticate standing when the enrollment phase was done sitting.
Again, using a background image, 30% of the participants could authenticate
sitting when the enrollment phase was done standing.
    Furthermore, we tested the viability of each combination of motion data,
consisting of gyroscope, accelerometer, and fusion features. Choosing only one
data set of them did work out well for user authentication, that is in 79–86% of
all test cases. However, an impostor was badly recognized, only in 83–88% of all
test cases. Indeed, the combination of all of them did work best, since a small
variation in input data can still be ignored.


7   Related Work

As mentioned in the introduction, to the best of our knowledge no work comes
with the capabilities of our authentication algorithm, even when leaving the
privacy part apart. However, there are many different fields which overlap with
techniques employed in SMAUG, also many biometric authentication schemes
were inspiring our design.
Signature Verification. The use of DTW for successful signature verification has
been considered in [11]. Follow-up work in this field copes with signature input
using a pen on handheld devices [20]. An overview of the current state of the art
of signature verification gives [10], while new approaches of SVM are combined
with signature verification in [9]. Good choices of features for signatures are
discussed in [21]. In [8], the authors investigate how signatures of the same
user alter over time. Finally, the work [13] focuses on signature verification on
smartphones.
Gesture Recognition. With respect to gesture recognition for single-touch ges-
tures, Rubine [24] is the usual reference when comparing new single-touch al-
gorithms. Another prominent example for single-touch and single-stroke gesture
recognition is $1 [38]. The authors of [1] present a very efficient follow-up work
for single-touch and multi-stroke. In short, the algorithm joins all strokes in all


                                        37
possible combinations and reduces the gesture recognition problem therefore to
the case of single-touch gestures. However, this algorithm family needs a pre-
defined set of gestures. The authors in [34] have developed a multi-dimension
DTW for gesture recognition. In [31], the authors present a gesture based user
authentication scheme for touch screens using solely the accelerometer. 3D hand
gesture recognition in the air with mobile devices and accelerometer is examined
in [3]. Similar research was done for Kinect and gesture recognition in [35], also
for Wii [18].
Sketch and Image Recognition. Sketches are drawings of simple symbols and
recognition is similar to gesture recognition. In [4], the authors classify differ-
ent sketches for technical drawings. The work [12] presents a sketch recognizer
for a defined set of sketches which are invariant for rotation. An algorithm for
recognition of multi-touch sketches is given in [30]. Also, [23] shows how image
recognition works and how it can be fooled.
Continuous Authentication. Continuous authentication means that the device
constantly tracks and evaluates the inputs and movements of the user onto the
device to authenticate the user. They generally suffer fom privacy loss in some
kind. Algorithms can be found in [17,5,25].
Mobile Authentication and Graphical Passwords. In [6], the authors present an
attack on the graphical password system of Windows 8. [7] gives an overview of
graphical password schemes developed so far. An enhancement for the Android
pattern authentication was presented in [19], which utilizes the accelerometer.
The authors of [33] give an authentication algorithm where up to five fingers
can be used for multi-touch single-stroke (per finger) in combination with touch
screen and accelerometer. Furthermore, they defined the adversary models for
mobile gesture recognition based on [35], which are all weaker than our adversary
model. In [32] the authors allow multi-touch and free-form gestures to measure
the amount of information of a gesture which can be used for authentication.
Finally, [26] presents a multi-touch authentication algorithm for five fingers using
touch screen data and a predefined gesture set. In [39] the authors test free-form
gesture authentication in non-labor environments.


8   Conclusion

We presented SMAUG, a novel authentication scheme for mobile devices based
using gestures. The gestures can be freely chosen, may consist of multiple strokes
of different length and can be drawn using multiple fingers at the same time.
Moreover, the layout of the gesture is freely chosen by the user and is in no
way restricted by SMAUG. To tie these gestures to a user, not only the form
of the gesture itself is considered but also how it has been entered. Hence, we
combine knowledge of the form of the gesture as well as the biometric properties
of its input resulting in a two-factor authentication. Data from different sensors
is collected to identify individual biometric properties of the user. We stress that
only sensors are used that are commonly found in mobile devices: touch screen,
gyroscope, and accelerometer. Compared to existing work, our scheme provides


                                        38
a significant higher level of flexibility and achieves security against the strongest
possible attacker – being non-continuous and without any loss of privacy at the
same time. In this work, we also provide an implementation which allowed us
to perform experiments with various participants. Our experiments show that
while legitimate users are correctly authenticated with overwhelming probability
of 99% after the third attempt, an attacker fails to impersonate the user with
overwhelming probability of 97%, even if he knows what gesture is required and
how it has to be entered. Hence, we believe that SMAUG provides an important
contribution towards solving the long-standing open problem of providing usable
and secure authentication for mobile devices.
    Although our scheme achieves already very good and reliable results, we see
several possible directions for further improvements. Our experiments were car-
ried out in labor environments, i.e., the user had a fixed pose during enrollment
and verification. In practice, one would require more robust schemes which on
the one hand also work while the user is moving, i.e., walking or driving, and
on the other hand need to respect rotation of the device, i.e., lying on a table
or when the user is lying. However, implementing this will reduce security. Cur-
rently SMAUG is strictly forcing the user to enter the strokes always in the
same order. While this makes sense in general, in some cases, e.g., when using
a pinch gesture, this may be hard to achieve. Here, a small time window which
allows for changes of the stroke order may be helpful. The user has to repeat
the gesture ten times during enrollment phase. One may argue that enrollment
should only rarely take place, but it would nonetheless be advantageous to re-
duce this number. Finally, as mentioned in the related work, biometric input
may alter over time due to the user’s natural behavior. Combining this with
SMAUG is an open problem which we plan to tackle in the future.


References
 1. L. Anthony and J. O. Wobbrock. $n-protractor: a fast and accurate multistroke
    recognizer. In S. Brooks and K. Hawkey, editors, Proceedings of the Graphics
    Interface 2012 Conference, GI ’12, Toronto, ON, Canada, May 28-30, 2012, pages
    117–120. ACM, 2012.
 2. A. J. Aviv, K. Gibson, E. Mossop, M. Blaze, and J. M. Smith. Smudge attacks
    on smartphone touch screens. In C. Miller and H. Shacham, editors, 4th USENIX
    Workshop on Offensive Technologies, WOOT ’10, Washington, D.C., USA, August
    9, 2010. USENIX Association, 2010.
 3. J. G. Casanova, C. S. Ávila, G. Bailador, and A. de Santos Sierra. Authentication
    in mobile devices through hand gesture recognition. Int. J. Inf. Sec., 11(2):65–83,
    2012.
 4. M. Field, S. Gordon, E. J. Peterson, R. Robinson, T. F. Stahovich, and C. Alvarado.
    The effect of task on classification accuracy: Using gesture recognition techniques
    in free-sketch recognition. Computers & Graphics, 34(5):499–512, 2010.
 5. M. Frank, R. Biedert, E. Ma, I. Martinovic, and D. Song. Touchalytics: On the
    applicability of touchscreen input as a behavioral biometric for continuous authen-
    tication. IEEE Transactions on Information Forensics and Security, 8(1):136–148,
    2013.


                                          39
 6. H. Gao, W. Jia, N. Liu, and K. Li. The hot-spots problem in windows 8 graph-
    ical password scheme. In G. Wang, I. Ray, D. Feng, and M. Rajarajan, editors,
    Cyberspace Safety and Security - 5th International Symposium, CSS 2013, Zhangji-
    ajie, China, November 13-15, 2013, Proceedings, volume 8300 of Lecture Notes in
    Computer Science, pages 349–362. Springer, 2013.
 7. H. Gao, W. Jia, F. Ye, and L. Ma. A survey on the use of graphical passwords in
    security. JSW, 8(7):1678–1698, 2013.
 8. M. Gomez-Barrero, J. Galbally, J. Fiérrez, J. Ortega-Garcia, and R. Plamondon.
    Variations of handwritten signatures with time: A sigma-lognormal analysis. In
    J. Fiérrez, A. Kumar, M. Vatsa, R. N. J. Veldhuis, and J. Ortega-Garcia, editors,
    International Conference on Biometrics, ICB 2013, 4-7 June, 2013, Madrid, Spain,
    pages 1–6. IEEE, 2013.
 9. C. Gruber, T. Gruber, S. Krinninger, and B. Sick. Online signature verification
    with support vector machines based on LCSS kernel functions. IEEE Transactions
    on Systems, Man, and Cybernetics, Part B, 40(4):1088–1100, 2010.
10. D. Impedovo and G. Pirlo. Automatic signature verification: The state of the art.
    IEEE Transactions on Systems, Man, and Cybernetics, Part C, 38(5):609–635,
    2008.
11. B. Kar, P. Dutta, T. Basu, C. Vielhauer, and J. Dittmann. Dtw based verification
    scheme of biometric signatures, 2006.
12. L. B. Kara and T. F. Stahovich. An image-based, trainable symbol recognizer for
    hand-drawn sketches. Computers & Graphics, 29(4):501–517, 2005.
13. R. P. Krish, J. Fiérrez, J. Galbally, and M. Martinez-Diaz. Dynamic signature
    verification on smart phones. In J. M. Corchado, J. Bajo, J. Kozlak, P. Pawlewski,
    J. M. Molina, V. Julián, R. A. Silveira, R. Unland, and S. Giroux, editors, High-
    lights on Practical Applications of Agents and Multi-Agent Systems - International
    Workshops of PAAMS 2013, Salamanca, Spain, May 22-24, 2013. Proceedings, vol-
    ume 365 of Communications in Computer and Information Science, pages 213–222.
    Springer, 2013.
14. J.     Krissler.         Chaos     computer     club   breaks    apple    touchid,
    http://www.ccc.de/en/updates/2013/ccc-breaks-apple-touchid, 2013.
15. J.    Krissler.         Hacking      putin’s   eyes:  How     to    bypass    bio-
    metrics      the    cheap     and      dirty    way    with     google     images,
    http://www.forbes.com/sites/thomasbrewster/2015/03/05/clone-putins-eyes-
    using-google-images/, 2015.
16. O. D. Lara and M. A. Labrador. A survey on human activity recognition using
    wearable sensors. IEEE Communications Surveys and Tutorials, 15(3):1192–1209,
    2013.
17. L. Li, X. Zhao, and G. Xue. Unobservable re-authentication for smartphones. In
    20th Annual Network and Distributed System Security Symposium, NDSS 2013,
    San Diego, California, USA, February 24-27, 2013. The Internet Society, 2013.
18. J. Liu, Z. Wang, L. Zhong, J. Wickramasuriya, and V. Vasudevan. uwave:
    Accelerometer-based personalized gesture recognition and its applications. In Sev-
    enth Annual IEEE International Conference on Pervasive Computing and Com-
    munications, PerCom 2009, 9-13 March 2009, Galveston, TX, USA, pages 1–9.
    IEEE Computer Society, 2009.
19. A. D. Luca, A. Hang, F. Brudy, C. Lindner, and H. Hussmann. Touch me once and
    i know it’s you!: implicit authentication based on touch screen patterns. In J. A.
    Konstan, E. H. Chi, and K. Höök, editors, CHI Conference on Human Factors
    in Computing Systems, CHI ’12, Austin, TX, USA - May 05 - 10, 2012, pages
    987–996. ACM, 2012.


                                         40
20. M. Martinez-Diaz, J. Fierrez, J. Galbally, F. Alonso-Fernandez, and J. Ortega-
    Garcia. Signature verification on handheld devices. In Proc. MADRINET Work-
    shop, pages 87–95, November 2007.
21. M. Martinez-Diaz, J. Fierrez, R. P. Krish, and J. Galbally. Mobile signature
    verification: Feature robustness and performance comparison. IET Biometrics,
    3(4):267–277, December 2014.
22. E. Miluzzo, A. Varshavsky, S. Balakrishnan, and R. R. Choudhury. Tapprints:
    your finger taps have fingerprints. In N. Davies, S. Seshan, and L. Zhong, editors,
    The 10th International Conference on Mobile Systems, Applications, and Services,
    MobiSys’12, Ambleside, United Kingdom - June 25 - 29, 2012, pages 323–336.
    ACM, 2012.
23. A. M. Nguyen, J. Yosinski, and J. Clune. Deep neural networks are easily fooled:
    High confidence predictions for unrecognizable images. CoRR, abs/1412.1897,
    2014.
24. D. Rubine. Specifying gestures by example. In J. J. Thomas, editor, Proceedings
    of the 18th Annual Conference on Computer Graphics and Interactive Techniques,
    SIGGRAPH 1991, pages 329–337. ACM, 1991.
25. M. Rybnicek, C. Lang-Muhr, and D. Haslinger. A roadmap to continuous bio-
    metric authentication on mobile devices. In M. Gerla, K. B. Letaief, G. C. Had-
    jichristofi, and C. Callegari, editors, International Wireless Communications and
    Mobile Computing Conference, IWCMC 2014, Nicosia, Cyprus, August 4-8, 2014,
    pages 122–127. IEEE, 2014.
26. N. Sae-Bae, K. Ahmed, K. Isbister, and N. D. Memon. Biometric-rich gestures:
    a novel approach to authentication on multi-touch devices. In J. A. Konstan,
    E. H. Chi, and K. Höök, editors, CHI Conference on Human Factors in Computing
    Systems, CHI ’12, Austin, TX, USA - May 05 - 10, 2012, pages 977–986. ACM,
    2012.
27. N. Sae-Bae, N. D. Memon, and K. Isbister. Investigating multi-touch gestures as a
    novel biometric modality. In IEEE Fifth International Conference on Biometrics:
    Theory, Applications and Systems, BTAS 2012, Arlington, VA, USA, September
    23-27, 2012, pages 156–161. IEEE, 2012.
28. N. Sae-Bae, N. D. Memon, K. Isbister, and K. Ahmed. Multitouch gesture-
    based authentication. IEEE Transactions on Information Forensics and Security,
    9(4):568–582, 2014.
29. H. Sakoe and S. Chiba. Readings in speech recognition. In A. Waibel and K.-
    F. Lee, editors, Readings in Speech Recognition, chapter Dynamic Programming
    Algorithm Optimization for Spoken Word Recognition, pages 159–165. Morgan
    Kaufmann Publishers Inc., San Francisco, CA, USA, 1990.
30. M. Schmidt and G. Weber. Recognition of multi-touch drawn sketches. In
    M. Kurosu, editor, Human-Computer Interaction. Interaction Modalities and Tech-
    niques - 15th International Conference, HCI International 2013, Las Vegas, NV,
    USA, July 21-26, 2013, Proceedings, Part IV, volume 8007 of Lecture Notes in
    Computer Science, pages 479–490. Springer, 2013.
31. M. Shahzad, A. X. Liu, and A. Samuel. Secure unlocking of mobile touch screen
    devices by simple gestures: you can see it but you can not do it. In S. Helal,
    R. Chandra, and R. Kravets, editors, The 19th Annual International Conference
    on Mobile Computing and Networking, MobiCom’13, Miami, FL, USA, September
    30 - October 04, 2013, pages 39–50. ACM, 2013.
32. M. Sherman, G. Clark, Y. Yang, S. Sugrim, A. Modig, J. Lindqvist, A. Oulasvirta,
    and T. Roos. User-generated free-form gestures for authentication: security and


                                          41
    memorability. In A. T. Campbell, D. Kotz, L. P. Cox, and Z. M. Mao, editors,
    The 12th Annual International Conference on Mobile Systems, Applications, and
    Services, MobiSys’14, Bretton Woods, NH, USA, June 16-19, 2014, pages 176–189.
    ACM, 2014.
33. J. Sun, R. Zhang, J. Zhang, and Y. Zhang. Touchin: Sightless two-factor authen-
    tication on multi-touch mobile devices. In IEEE Conference on Communications
    and Network Security, CNS 2014, San Francisco, CA, USA, October 29-31, 2014,
    pages 436–444. IEEE, 2014.
34. G. A. ten Holt, M. J. Reinders, and E. A. Hendriks. Multi-dimensional dynamic
    time warping for gesture recognition. In Thirteenth annual conference of the Ad-
    vanced School for Computing and Imaging, June 13-15 2007.
35. J. Tian, C. Qu, W. Xu, and S. Wang. Kinwrite: Handwriting-based authentication
    using kinect. In 20th Annual Network and Distributed System Security Symposium,
    NDSS 2013, San Diego, California, USA, February 24-27, 2013. The Internet So-
    ciety, 2013.
36. M. Trojahn. Multi-factor authentication of mobile devices (smartphones and
    tablets), 2012.
37. R. Weiss and A. D. Luca. Passshapes: utilizing stroke based authentication to in-
    crease password memorability. In A. Gulz, C. Magnusson, L. Malmborg, H. Eftring,
    B. Jönsson, and K. Tollmar, editors, Proceedings of the 5th Nordic Conference on
    Human-Computer Interaction 2008, Lund, Sweden, October 20-22, 2008, volume
    358 of ACM International Conference Proceeding Series, pages 383–392. ACM,
    2008.
38. J. O. Wobbrock, A. D. Wilson, and Y. Li. Gestures without libraries, toolkits or
    training: a $1 recognizer for user interface prototypes. In C. Shen, R. J. K. Jacob,
    and R. Balakrishnan, editors, Proceedings of the 20th Annual ACM Symposium
    on User Interface Software and Technology, Newport, Rhode Island, USA, October
    7-10, 2007, pages 159–168. ACM, 2007.
39. Y. Yang, G. D. Clark, J. Lindqvist, and A. Oulasvirta. Free-form gesture authenti-
    cation in the wild. In J. Kaye, A. Druin, C. Lampe, D. Morris, and J. P. Hourcade,
    editors, Proceedings of the 2016 CHI Conference on Human Factors in Computing
    Systems, San Jose, CA, USA, May 7-12, 2016, pages 3722–3735. ACM, 2016.
40. Y. Zhang and T. Wei. Fingerprints on mobile devices: Abusing and leaking,
    https://www.blackhat.com/docs/us-15/materials/us-15-zhang-fingerprints-on-
    mobile-devices-abusing-and-leaking.pdf, 2015.
41. N. Zheng, K. Bai, H. Huang, and H. Wang. You are how you touch: User verification
    on smartphones via tapping behaviors. In 22nd IEEE International Conference on
    Network Protocols, ICNP 2014, Raleigh, NC, USA, October 21-24, 2014, pages
    221–232. IEEE Computer Society, 2014.
42. J. Zhu, P. Wu, X. Wang, and J. Zhang. Sensec: Mobile security through passive
    sensing. In International Conference on Computing, Networking and Communica-
    tions, ICNC 2013, San Diego, CA, USA, January 28-31, 2013, pages 1128–1133.
    IEEE Computer Society, 2013.




                                          42
              Multi- Free-Form Multi- Multi- Multi-                Multi-     Sensor Graphical Continuous Operating Sample
         Work Gesture Gesture  Touch Stroke Factor                 Sensor     Fusion Password Authentication System  Size

          [11]      no         yes        no        yes     no      no (T)      no       no             no           some         20
          [37]      no         no         no        no      no      no (T)      no       no             no            any         24
          [18]      no         yes        no        no      no      no (A)      no       no             no            Wii          ?
          [27]      no         no      yes (5fix)   no      no      no (T)      no       no             no            any         10
          [26]      no         yes     yes (5fix)   no      no      no (T)      no       no             no            any         10
          [19]      no         no      partly (2)   no      yes    yes (TA)     no       no             no          Android       20
          [42]     partly      no         no        yes     no    yes (AGM)     no       no             yes         modified       –
          [31]      yes        no       yes (5)     no      no     yes (TA)     no       no             no            any         25
          [17]     partly      no         no        yes     no      no (T)      no       no             yes         modified       –
43
          [28]      no         yes      yes (5)     no      no      no (T)      no       yes            no            any         10
          [32]      no         yes      yes (4)     no      no      no (T)      no       no             no            any         10
          [33]      no         yes      yes (5)     no      no     yes (TA)     no       no             no            any        20-50
          [41]      no         no         no        no      yes    yes (TA)     no       no             no            any         20

        SMAUG       yes        yes     yes (10+)    yes     yes   yes (TAG)    yes       yes             no            any        10

     Table 6. Overview and comparison of mobile authentication schemes and their features. For multi-touch, the number in brackets denotes
     the possible maximum numbers of fingers used at the same time, “fix” means that this exact amount of fingers must be used For multi-
     sensor, T denotes touch sensor, A accelerometer, G gyroscope, and M magnetometer. A (red) “no” denotes that the work of this specific
     row lacks this feature, and vice versa a (green) “yes” displays that this feature is fully supported. The colors of “yes” and “no” are
     interchanged for continuous authentication due to the reduced privacy. As the table shows, our algorithm SMAUG is rich on features
     and steps ahead of existing work, providing the richest set of capabilities.
