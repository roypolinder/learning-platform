//! Texts of account emails in the learner's language.

use super::OutgoingEmail;

#[derive(Clone, Copy)]
enum Lang {
    En,
    Nl,
}

impl Lang {
    fn of(locale: &str) -> Self {
        if locale == "nl" { Self::Nl } else { Self::En }
    }
}

fn email(to: &str, subject: &str, body: String) -> OutgoingEmail {
    OutgoingEmail {
        to: to.to_owned(),
        subject: subject.to_owned(),
        body,
    }
}

const SIGNATURE: &str = "— Learning Platform";

pub fn verify_email(locale: &str, to: &str, name: &str, link: &str) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Confirm your email address",
            format!(
                "Hi {name},\n\nConfirm that this is your email address by opening this link:\n\n{link}\n\n\
                 The link works for 48 hours. Didn't create an account? Then you can ignore this email.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Bevestig je e-mailadres",
            format!(
                "Hoi {name},\n\nBevestig dat dit jouw e-mailadres is door deze link te openen:\n\n{link}\n\n\
                 De link werkt 48 uur. Heb je geen account aangemaakt? Dan kun je deze e-mail negeren.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}

pub fn reset_password(locale: &str, to: &str, name: &str, link: &str) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Reset your password",
            format!(
                "Hi {name},\n\nSomeone asked to reset the password of your account. Choose a new password here:\n\n{link}\n\n\
                 The link works for 1 hour and only once. Didn't ask for this? Then ignore this email; your password stays the same.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Stel je wachtwoord opnieuw in",
            format!(
                "Hoi {name},\n\nIemand heeft gevraagd om het wachtwoord van je account opnieuw in te stellen. Kies hier een nieuw wachtwoord:\n\n{link}\n\n\
                 De link werkt 1 uur en maar één keer. Heb je dit niet gevraagd? Negeer deze e-mail dan; je wachtwoord blijft hetzelfde.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}

/// Sent to the new address; the change happens only when this link is opened.
pub fn confirm_new_email(locale: &str, to: &str, name: &str, link: &str) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Confirm your new email address",
            format!(
                "Hi {name},\n\nYou asked to use this address for your account from now on. Confirm it by opening this link:\n\n{link}\n\n\
                 The link works for 24 hours. Until then your account keeps its current address.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Bevestig je nieuwe e-mailadres",
            format!(
                "Hoi {name},\n\nJe hebt gevraagd om voortaan dit adres voor je account te gebruiken. Bevestig het door deze link te openen:\n\n{link}\n\n\
                 De link werkt 24 uur. Tot die tijd houdt je account het huidige adres.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}

/// Sent to the current address, so a hijacked session cannot move the account silently.
pub fn email_change_requested(
    locale: &str,
    to: &str,
    name: &str,
    new_email: &str,
) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Your email address is being changed",
            format!(
                "Hi {name},\n\nSomeone asked to change the email address of your account to {new_email}. \
                 It changes once that address is confirmed.\n\n\
                 Wasn't this you? Sign in, cancel the change in Settings and change your password.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Je e-mailadres wordt gewijzigd",
            format!(
                "Hoi {name},\n\nIemand heeft gevraagd om het e-mailadres van je account te wijzigen in {new_email}. \
                 Het verandert zodra dat adres is bevestigd.\n\n\
                 Was jij dit niet? Log in, annuleer de wijziging bij Instellingen en wijzig je wachtwoord.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}

pub fn password_changed(locale: &str, to: &str, name: &str) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Your password was changed",
            format!(
                "Hi {name},\n\nThe password of your account was just changed, and other devices were signed out.\n\n\
                 Wasn't this you? Reset your password right away with \"Forgot password?\" on the sign-in page.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Je wachtwoord is gewijzigd",
            format!(
                "Hoi {name},\n\nHet wachtwoord van je account is zojuist gewijzigd, en andere apparaten zijn uitgelogd.\n\n\
                 Was jij dit niet? Stel je wachtwoord meteen opnieuw in via \"Wachtwoord vergeten?\" op de inlogpagina.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}

pub fn account_deleted(locale: &str, to: &str, name: &str) -> OutgoingEmail {
    let lang = Lang::of(locale);
    match lang {
        Lang::En => email(
            to,
            "Your account was deleted",
            format!(
                "Hi {name},\n\nYour account and everything in it (progress, notes, project and agent connections) \
                 have been deleted. You won't get any more emails from us.\n\n{SIGNATURE}\n"
            ),
        ),
        Lang::Nl => email(
            to,
            "Je account is verwijderd",
            format!(
                "Hoi {name},\n\nJe account en alles daarin (voortgang, notities, project en agentkoppelingen) \
                 zijn verwijderd. Je krijgt geen e-mails meer van ons.\n\n{SIGNATURE}\n"
            ),
        ),
    }
}
