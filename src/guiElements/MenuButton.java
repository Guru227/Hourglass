package guiElements;

import javax.swing.JButton;

import constants.CurrentConfiguration;

public class MenuButton extends JButton
{
	/**
	 * 
	 */
	private static final long serialVersionUID = -7026716914176317123L;

	MenuButton(){
		setBounds(0,0,CurrentConfiguration.buttonFontSize,CurrentConfiguration.buttonFontSize);	
		
		//customize look and feel
		setBorderPainted(false);	//removes borders
		setFocusPainted(false);		//removes default color
		setFont(CurrentConfiguration.buttonFont);
		
		setBackground(CurrentConfiguration.buttonBg);
        setForeground(CurrentConfiguration.buttonFg);
        System.out.println(CurrentConfiguration.buttonBg);
        System.out.println(CurrentConfiguration.buttonFg);
        //Hover effects
        addMouseListener(new java.awt.event.MouseAdapter() {
	        public void mouseEntered(java.awt.event.MouseEvent evt) {
	        	setBackground(CurrentConfiguration.buttonHoverBg);
	        }

	        public void mouseExited(java.awt.event.MouseEvent evt) {
	            setBackground(CurrentConfiguration.buttonBg);
	        }
	    }); 
	}
}
