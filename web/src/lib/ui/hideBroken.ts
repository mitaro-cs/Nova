// Битая картинка не должна показывать значок ошибки: прячем (под ней - фон или заглушка).
export function hideBroken(img: HTMLImageElement) {
  const fail = () => {img.style.visibility='hidden';};
  img.addEventListener('error', fail);
  if(img.complete&&!img.naturalWidth&&img.src)fail();
  return {destroy(){img.removeEventListener('error',fail);}};
}
