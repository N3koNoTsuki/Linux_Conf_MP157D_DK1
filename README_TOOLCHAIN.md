# README — Création de la toolchain de cross-compilation

## Introduction

Ce document décrit la procédure de construction d'une toolchain de cross-compilation pour la carte STM32MP157A-DK1 (Cortex-A7, ARMv7), en suivant le TP officiel Bootlin *Embedded Linux system development — STM32MP157 Discovery Kit variant* (édition d'avril 2026).

La toolchain est l'ensemble des outils (compilateur, éditeur de liens, bibliothèque C, en-têtes du noyau) qui permet, depuis un PC x86_64, de produire des binaires exécutables sur la cible ARM. Elle est construite à l'aide de **crosstool-ng**, qui automatise le téléchargement et la compilation cohérente de tous ces composants.

Pour notre projet, on retient les choix Bootlin sauf sur deux points personnalisés :

- **vendor string** : `Neko` au lieu de `training` (pour identifier nos binaires)
- **alias** : `arm-Neko-linux` au lieu de `arm-linux`

Ces deux changements donnent un préfixe final `arm-Neko-linux-musleafih-` au lieu du `arm-training-linux-musleabihf-` du tuto officiel. Tout le reste (CPU, FPU, libc, version de gcc) suit le tuto Bootlin tel quel.

## Sommaire

1. [Prérequis](#1-prérequis)
2. [Récupération de crosstool-ng](#2-récupération-de-crosstool-ng)
3. [Compilation et installation locale de crosstool-ng](#3-compilation-et-installation-locale-de-crosstool-ng)
4. [Configuration de la toolchain](#4-configuration-de-la-toolchain)
5. [Construction de la toolchain](#5-construction-de-la-toolchain)
6. [Test de la toolchain](#6-test-de-la-toolchain)
7. [Nettoyage (optionnel)](#7-nettoyage-optionnel)

---

## 1. Prérequis

- Un système Linux (ou VM) avec **au moins 4 Go de RAM**.
- Une connexion Internet pour télécharger les sources (gcc, binutils, musl, en-têtes kernel, etc. — quelques centaines de Mo au total).
- Environ **9 Go d'espace disque** pendant la compilation (récupérables ensuite par `./ct-ng clean`).

Installer les paquets nécessaires (sur Ubuntu/Debian) :

```bash
sudo apt install build-essential git autoconf bison flex texinfo help2man gawk \
    libtool-bin libncurses5-dev unzip gettext python3 rsync
```

---

## 2. Récupération de crosstool-ng

On clone le dépôt git officiel et on se positionne sur le tag testé par Bootlin :

```bash
git clone https://github.com/crosstool-ng/crosstool-ng
cd crosstool-ng/
git checkout crosstool-ng-1.28.0
```

---

## 3. Compilation et installation locale de crosstool-ng

Comme on construit crosstool-ng depuis le dépôt git (et non depuis une archive de release), il faut d'abord générer le script `configure` et tous les fichiers normalement présents dans une archive :

```bash
./bootstrap
```

Deux options sont possibles : installer crosstool-ng globalement sur le système, ou le garder dans son dossier de téléchargement. On choisit la seconde option, plus propre :

```bash
./configure --enable-local
make
```

Une fois compilé, on peut afficher l'aide pour vérifier que tout fonctionne :

```bash
./ct-ng help
```

---

## 4. Configuration de la toolchain

Une seule installation de crosstool-ng permet de produire autant de toolchains que voulu, pour différentes architectures, libc et versions de composants. crosstool-ng fournit des **samples** prêts à l'emploi qu'on peut lister :

```bash
./ct-ng list-samples
```

On part du sample **Cortex A5** (crosstool-ng n'a pas de sample Cortex A7 prêt) :

```bash
./ct-ng <nom-du-sample-cortex-a5>
```

> *Astuce : la commande exacte est celle indiquée par `list-samples` pour la ligne Cortex A5. Le sample sert juste de point de départ, on l'ajuste dans `menuconfig` juste après.*

On lance ensuite l'interface de configuration pour affiner :

```bash
./ct-ng menuconfig
```

### Options à régler

**Dans `Path and misc options` :**
- Activer `Try features marked as EXPERIMENTAL` si ce n'est pas déjà fait.

**Dans `Target options` :**
- `Emit assembly for CPU (ARCH_CPU)` → **`cortex-a7`**
- `Use specific FPU (ARCH_FPU)` → **`vfpv4`**
- `Floating point` → **`hardware (FPU)`**

**Dans `Toolchain options` :**
- `Tuple's vendor string (TARGET_VENDOR)` → **`Neko`**
  *(Bootlin utilise `training` ; on personnalise pour identifier nos binaires.)*
- `Tuple's alias (TARGET_ALIAS)` → **`arm-Neko-linux`**
  *(Bootlin utilise `arm-linux` ; on adapte en cohérence avec le vendor.)*

**Dans `Operating System` :**
- `Version of linux` → la version la plus proche **mais antérieure ou égale à 6.12**.
  C'est important : les en-têtes kernel embarquées dans la toolchain ne doivent pas être plus récentes que le kernel qui tournera sur la carte, sans quoi la libc pourrait référencer des appels système absents du kernel cible.

**Dans `C-library` :**
- `C library` → **`musl (LIBC_MUSL)`**
- Conserver la version proposée par défaut.

**Dans `C compiler` :**
- `Version of gcc` → **`14.3.0`**
- Vérifier que `C++ (CC_LANG_CXX)` est activé.

**Dans `Debug facilities` :**
- Tout désactiver. Les outils de debug peuvent être fournis plus tard par les outils de build du rootfs (BusyBox, Buildroot…).

> *Important : ces réglages sont ceux testés par Bootlin. S'écarter de cette configuration peut faire perdre du temps sur des problèmes inattendus.*

---

## 5. Construction de la toolchain

Une fois la configuration sauvegardée, la construction tient en une commande :

```bash
./ct-ng build
```

La toolchain est installée par défaut dans **`$HOME/x-tools/`**. Pour notre configuration, le dossier final s'appelle :

```
$HOME/x-tools/arm-Neko-linux-musleafih/
```

> ⚠️ La compilation peut prendre **30 minutes à plusieurs heures** selon la machine. Elle télécharge et compile gcc, binutils, musl et les en-têtes kernel.

---

## 6. Test de la toolchain

On ajoute le dossier `bin` de la toolchain au `PATH` :

```bash
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
```

On vérifie que le compilateur répond :

```bash
arm-Neko-linux-musleafih-gcc --version
```

On compile un petit `hello.c` et on vérifie que la cible est bien ARM :

```bash
arm-Neko-linux-musleafih-gcc -o hello hello.c
file hello
```

La commande `file` doit indiquer un binaire **ELF 32-bit LSB executable, ARM, EABI5**.

### (Optionnel) Exécuter le binaire ARM sur le PC x86 avec QEMU user

```bash
sudo apt install qemu-user
qemu-arm hello
```

Si on obtient une erreur `Could not open '/lib/ld-musl-armhf.so.1'`, c'est que QEMU ne trouve pas le linker dynamique. On le pointe vers le sysroot de la toolchain :

```bash
qemu-arm -L $HOME/x-tools/arm-Neko-linux-musleafih/arm-Neko-linux-musleafih/sysroot hello
```

On doit voir s'afficher `Hello world!`.

---

## 7. Nettoyage (optionnel)

Si on est limité en espace disque, on peut récupérer environ **9 Go** en supprimant les sources et fichiers intermédiaires (la toolchain installée dans `$HOME/x-tools` n'est pas touchée) :

```bash
./ct-ng clean
```

À ne faire que si on est sûr de la configuration : en cas d'erreur, refaire un `./ct-ng build` repartira de zéro.

---

## Récapitulatif des chemins

| Élément                          | Chemin                                                              |
| -------------------------------- | ------------------------------------------------------------------- |
| Sources de crosstool-ng          | `./crosstool-ng/`                                                   |
| Toolchain installée              | `$HOME/x-tools/arm-Neko-linux-musleafih/`                           |
| Préfixe des outils               | `arm-Neko-linux-musleafih-` (gcc, ld, ar, objdump…)                 |
| Sysroot (libs et headers cibles) | `$HOME/x-tools/arm-Neko-linux-musleafih/arm-Neko-linux-musleafih/sysroot/` |

## Utilisation dans la suite du projet

Pour les compilations qui suivent (U-Boot, TF-A, kernel Linux, modules out-of-tree, programmes utilisateur), on positionne typiquement :

```bash
export PATH="$HOME/x-tools/arm-Neko-linux-musleafih/bin:$PATH"
export ARCH=arm
export CROSS_COMPILE=arm-Neko-linux-musleafih-
```

C'est tout ce qu'il faut pour que `make` du noyau, du bootloader ou d'un module out-of-tree pointe vers notre toolchain.

---

## Sources

- Bootlin — *Embedded Linux system development, STM32MP157 Discovery Kit variant, Practical Labs*, avril 2026 : <https://bootlin.com/doc/training/embedded-linux/embedded-linux-stm32mp1-labs.pdf> (chapitre *Building a cross-compiling toolchain*).
- Documentation crosstool-ng : <https://crosstool-ng.github.io/docs/install/#hackers-way>
- Dépôt du projet : <https://github.com/N3koNoTsuki/Linux_Conf_MP157D_DK1>
