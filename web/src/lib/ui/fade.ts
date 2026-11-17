// Картинка появляется плавно, когда загрузится (а не «выскакивает» кусками). Уже загруженная
// (из кеша) - сразу, без мигания. Только opacity - видеокарте легко.
export function fadeIn(img: HTMLImageElement) {
  if (!img.complete)img.style.opacity = '0';
  img.style.transition = 'opacity 0.45s ease';
  const show = () => {img.style.opacity = '1';img.style.visibility='';};
  const fail = () => {img.style.opacity='0';img.style.visibility='hidden';};
  img.addEventListener('load', show);
  img.addEventListener('error', fail);
  if(img.complete){if(img.naturalWidth)show();else fail();}
  return {destroy(){img.removeEventListener('load',show);img.removeEventListener('error',fail);}};
}
