export const springs = {
  feedback: { type: 'spring' as const, stiffness: 500, damping: 35, mass: .65 },
  content: { type: 'spring' as const, stiffness: 380, damping: 30, mass: .8 },
  shape: { type: 'spring' as const, stiffness: 320, damping: 28, mass: .9 },
};
export const reveal = { initial: { opacity: 0, y: 7 }, animate: { opacity: 1, y: 0 }, exit: { opacity: 0, y: -4 }, transition: { duration: .16 } };
