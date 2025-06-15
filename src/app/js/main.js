document.onreadystatechange = () => {
  if (document.readyState === "complete") {
    document.body.appendChild(document.createElement("h3")).textContent = "Yo this loaded"
  }
}