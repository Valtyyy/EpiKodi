/**
 * Focus indicator for the TV. `focus:` and not `focus-visible:`: focus moved by script
 * (spatial navigation, phone remote) does not always match `:focus-visible`, and the
 * focus must stay visible from 3 meters in every case (ARCHITECTURE.md "Styles avec Tailwind").
 */
export const focusRing = "focus:outline-4 focus:outline-offset-2 focus:outline-focus";
