// SPDX-License-Identifier: GPL-2.0

use kernel::prelude::*;
use kernel::c_str;
use kernel::miscdevice::{MiscDeviceOptions, MiscDeviceRegistration, MiscDevice};
use kernel::alloc::{KBox, flags::GFP_KERNEL}; // KBox = Box<_, Kmalloc>

// utile pour les read_iter/write_iter
use kernel::fs::Kiocb;
use kernel::iov::IovIterDest;

// ----- Déclaration du module -----
module! {
    type: Ds3231Module,
    name: "ds3231",
    authors: ["Quentin"],
    description: "Hello Rust out-of-tree driver",
    license: "GPL",
}

// ----- Type "ops" : callbacks du misc device -----
struct Ds3231Ops;

impl MiscDevice for Ds3231Ops {
    // 1) Associated type correcte
    type Ptr = ();

    // 2) Const requise par la vtable (valeur par défaut vide)
    const USE_VTABLE_ATTR: () = ();
    const HAS_READ_ITER: bool = true;

    // 3) Signature open : 2 paramètres (fichier ET registration)
    fn open(_file: &kernel::fs::File, _dev: &MiscDeviceRegistration<Self>) -> Result<Self::Ptr> {
        // Impl réelle à venir
        Ok(())
    }

    // Tu pourras ajouter ioctl/read/write/… ici.
    fn read_iter(mut kiocb: Kiocb<'_, Self::Ptr>, iov: &mut IovIterDest<'_>) -> Result<usize> {
        // Contenu “virtuel” du fichier
        static DATA: &[u8] = b"hello from Ds3231\n";        
        // En 7.0‑rc1, `ki_pos_mut()` renvoie &mut i64.        
        // `simple_read_from_buffer` écrit la bonne sous‑slice en fonction de la position,        
        // et met à jour la position pour les lectures suivantes.        
        let read = iov.simple_read_from_buffer(kiocb.ki_pos_mut(), DATA)?;        
        Ok(read)    
    }
}

// ----- Type de module : garde la registration en vie -----
struct Ds3231Module {
    _reg: core::pin::Pin<KBox<MiscDeviceRegistration<Ds3231Ops>>>,
}

impl kernel::Module for Ds3231Module {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        pr_info!("Hello from out-of-tree driver!\n");

        // Nom du device dans /dev
        let opts = MiscDeviceOptions {
            name: c_str!("my_misc_device"),
        };

        // Enregistre le misc device :
        // - register(opts) -> impl PinInit<_, Error>
        // - KBox::pin_init(..., GFP_KERNEL) -> Result<Pin<KBox<_>>, Error>
        let reg = KBox::pin_init(MiscDeviceRegistration::<Ds3231Ops>::register(opts), GFP_KERNEL)?;

        Ok(Self { _reg: reg })
    }
}

impl Drop for Ds3231Module {
    fn drop(&mut self) {
        // A la destruction, Drop de _reg appelle misc_deregister() automatiquement.
        pr_info!("ds3231: exit()\n");
    }
}