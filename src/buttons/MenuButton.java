package buttons;

import java.awt.Color;
import java.awt.Dimension;
import java.awt.Font;

import javax.swing.JButton;

import constants.Constants;
import constants.CurrentConfiguration;

public class MenuButton extends JButton
{
	MenuButton(){
		setBounds(0,0,Constants.buttonFontSize,Constants.buttonFontSize);	
		
		//customize look and feel
		setBorderPainted(false);	//removes borders
		setFocusPainted(false);		//removes default color
		setFont(Constants.buttonFont);
		
		if(CurrentConfiguration.darkMode) {
			setBackground(Constants.darkModeButtonBg);
	        setForeground(Constants.darkModeButtonFg);
	        //Hover effects
	        addMouseListener(new java.awt.event.MouseAdapter() {
		        public void mouseEntered(java.awt.event.MouseEvent evt) {
		        	setBackground(Constants.darkModeButtonHoverBg);
		        }

		        public void mouseExited(java.awt.event.MouseEvent evt) {
		            setBackground(Constants.darkModeButtonBg);
		        }
		    });
		}
		else {
			setBackground(Color.WHITE);
	        setForeground(Color.BLACK);
	        addMouseListener(new java.awt.event.MouseAdapter() {
		        public void mouseEntered(java.awt.event.MouseEvent evt) {
		        	setBackground(Color.decode("0xDDDDDD"));
		        }
	
		        public void mouseExited(java.awt.event.MouseEvent evt) {
		            setBackground(Color.decode("0xFFFFFF"));
		        }
		    });
		}   
	}
}
